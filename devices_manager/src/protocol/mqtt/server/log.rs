use super::{Ack, DataRequest, FilterIdx, SubscriptionMeter};
use slab::Slab;
use tracing::{info, trace, warn};

use crate::protocol::mqtt::version::{
    matches, ConnAck, ConnAckProperties, PingResp, PubAck, PubComp, PubRec, PubRel, Publish,
    PublishProperties, SubAck, UnsubAck,
};

use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::Instant;
use crate::protocol::mqtt::{ConnectionId, Cursor, Filter, Offset, Topic};
use crate::protocol::mqtt::server::router::RouterConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Position {
    Next { start: (u64, u64), end: (u64, u64) },
    Done { start: (u64, u64), end: (u64, u64) },
}


pub trait Storage {
    fn size(&self) -> usize;
}

type PubWithProp = (Publish, Option<PublishProperties>);

#[derive(Clone)]
pub struct PublishData {
    pub publish: Publish,
    pub properties: Option<PublishProperties>,
    pub timestamp: Instant,
}

impl From<PubWithProp> for PublishData {
    fn from((publish, properties): PubWithProp) -> Self {
        PublishData {
            publish,
            properties,
            timestamp: Instant::now(),
        }
    }
}

// TODO: remove this from here
impl Storage for PublishData {
    // TODO: calculate size of publish properties as well!
    fn size(&self) -> usize {
        let publish = &self.publish;
        4 + publish.topic.len() + publish.payload.len()
    }
}

#[derive(Debug)]
pub struct Waiters<T> {
    /// Waiters on new topics
    current: VecDeque<(ConnectionId, T)>,
}

impl<T> Waiters<T> {
    pub fn with_capacity(max_connections: usize) -> Waiters<T> {
        Waiters {
            current: VecDeque::with_capacity(max_connections),
        }
    }

    /// Current parked connection requests waiting for new data
    pub fn waiters(&self) -> &VecDeque<(ConnectionId, T)> {
        &self.current
    }

    /// Pushes a request to current wait queue
    pub fn register(&mut self, id: ConnectionId, request: T) {
        let request = (id, request);
        self.current.push_back(request);
    }

    /// Swaps next wait queue with current wait queue
    pub fn take(&mut self) -> Option<VecDeque<(ConnectionId, T)>> {
        if self.current.is_empty() {
            return None;
        }

        let next = VecDeque::new();
        Some(std::mem::replace(&mut self.current, next))
    }

    /// Remove a connection from waiters
    pub fn remove(&mut self, id: ConnectionId) -> Vec<T> {
        let mut requests = Vec::new();

        while let Some(index) = self.current.iter().position(|x| x.0 == id) {
            let request = self.current.swap_remove_back(index).map(|v| v.1).unwrap();
            requests.push(request)
        }

        requests
    }

    pub fn get_mut(&mut self) -> &mut VecDeque<(ConnectionId, T)> {
        &mut self.current
    }
}


/// Stores 'device' data and 'actions' data in native commitlog
/// organized by subscription filter. Device data is replicated
/// while actions data is not
pub struct DataLog {
    pub config: RouterConfig,
    /// Native commitlog data organized by subscription. Contains
    /// device data and actions data logs.
    ///
    /// Device data is replicated while actions data is not.
    /// Also has waiters used to wake connections/replicator tracker
    /// which are caught up with all the data on 'Filter' and waiting
    /// for new data
    pub native: Slab<Data<PublishData>>,
    /// Map of subscription filter name to filter index
    filter_indexes: HashMap<Filter, FilterIdx>,
    retained_publishes: HashMap<Topic, PublishData>,
    /// List of filters associated with a topic
    publish_filters: HashMap<Topic, Vec<FilterIdx>>,
}

impl DataLog {
    pub fn new(config: RouterConfig) -> io::Result<DataLog> {
        let mut native = Slab::new();
        let mut filter_indexes = HashMap::new();
        let retained_publishes = HashMap::new();
        let publish_filters = HashMap::new();

        if let Some(warmup_filters) = config.initialized_filters.clone() {
            for filter in warmup_filters {
                let data = Data::new(&filter, &config);

                // Add commitlog to datalog and add datalog index to filter to
                // datalog index map
                let idx = native.insert(data);
                filter_indexes.insert(filter, idx);
            }
        }

        Ok(DataLog {
            config,
            native,
            publish_filters,
            filter_indexes,
            retained_publishes,
        })
    }

    pub fn meter(&mut self, filter: &str) -> Option<&mut SubscriptionMeter> {
        let data = self.native.get_mut(*self.filter_indexes.get(filter)?)?;
        Some(&mut data.meter)
    }

    pub fn waiters(&self, filter: &Filter) -> Option<&Waiters<DataRequest>> {
        self.native
            .get(*self.filter_indexes.get(filter)?)
            .map(|data| &data.waiters)
    }

    pub fn remove_waiters_for_id(
        &mut self,
        id: ConnectionId,
        filter: &Filter,
    ) -> Option<DataRequest> {
        let data = self
            .native
            .get_mut(*self.filter_indexes.get(filter)?)
            .unwrap();
        let waiters = data.waiters.get_mut();

        waiters
            .iter()
            .position(|&(conn_id, _)| conn_id == id)
            .and_then(|index| {
                waiters
                    .swap_remove_back(index)
                    .map(|(_, data_req)| data_req)
            })
    }

    // TODO: Currently returning a Option<Vec> instead of Option<&Vec> due to Rust borrow checker
    // limitation
    pub fn matches(&mut self, topic: &str) -> Option<Vec<usize>> {
        match &self.publish_filters.get(topic) {
            Some(v) => Some(v.to_vec()),
            None => {
                let v: Vec<usize> = self
                    .filter_indexes
                    .iter()
                    .filter(|(filter, _)| matches(topic, filter))
                    .map(|(_, filter_idx)| *filter_idx)
                    .collect();

                if !v.is_empty() {
                    self.publish_filters.insert(topic.to_owned(), v.clone());
                }

                Some(v)
            }
        }
    }

    pub fn next_native_offset(&mut self, filter: &str) -> (FilterIdx, Offset) {
        let publish_filters = &mut self.publish_filters;
        let filter_indexes = &mut self.filter_indexes;

        let (filter_idx, data) = match filter_indexes.get(filter) {
            Some(idx) => (*idx, self.native.get(*idx).unwrap()),
            None => {
                let data = Data::new(filter, &self.config);

                // Add commitlog to datalog and add datalog index to filter to
                // datalog index map
                let idx = self.native.insert(data);
                self.filter_indexes.insert(filter.to_owned(), idx);

                // Match new filter to existing topics and add to publish_filters if it matches
                for (topic, filters) in publish_filters.iter_mut() {
                    if matches(topic, filter) {
                        filters.push(idx);
                    }
                }

                (idx, self.native.get(idx).unwrap())
            }
        };

        (filter_idx, data.log.next_offset())
    }

    pub fn native_readv(
        &self,
        filter_idx: FilterIdx,
        offset: Offset,
        len: u64,
    ) -> io::Result<(Position, Vec<(PubWithProp, Offset)>)> {
        // unwrap to get index of `self.native` is fine here, because when a new subscribe packet
        // arrives in `Router::handle_device_payload`, it first calls the function
        // `next_native_offset` which creates a new commitlog if one doesn't exist. So any new
        // reads will definitely happen on a valid filter.
        let data = self.native.get(filter_idx).unwrap();
        let mut o = Vec::new();
        // TODO: `readv` is infallible but its current return type does not
        // reflect that. Consequently, this method is also infallible.
        // Encoding this information is important so that calling function
        // has more information on how this method behaves.
        let next = data.log.readv(offset, len, &mut o)?;

        let now = Instant::now();
        o.retain_mut(|(pubdata, _)| {
            // Keep data if no properties exists, which implies no message expiry!
            let Some(properties) = pubdata.properties.as_mut() else {
                return true;
            };

            // Keep data if there is no message_expiry_interval
            let Some(message_expiry_interval) = properties.message_expiry_interval.as_mut() else {
                return true;
            };

            let time_spent = (now - pubdata.timestamp).as_secs() as u32;

            let is_valid = time_spent < *message_expiry_interval;

            // ignore expired messages
            if is_valid {
                // set message_expiry_interval to (original value - time spent waiting in server)
                // ref: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901112
                *message_expiry_interval -= time_spent;
            }

            is_valid
        });

        // no need to include timestamp when returning
        let o = o
            .into_iter()
            .map(|(pubdata, offset)| ((pubdata.publish, pubdata.properties), offset))
            .collect();

        Ok((next, o))
    }

    pub fn shadow(&mut self, filter: &str) -> Option<PubWithProp> {
        let data = self.native.get_mut(*self.filter_indexes.get(filter)?)?;
        data.log.last().map(|p| (p.publish, p.properties))
    }

    /// This method is called when the subscriber has caught up with the commit log. In which case,
    /// instead of actively checking for commits in each `Router::run_inner` iteration, we instead
    /// wait and only try reading again when new messages have been added to the commit log. This
    /// methods converts a `DataRequest` (which actively reads the commit log in `Router::consume`)
    /// to a `Waiter` (which only reads when notified).
    pub fn park(&mut self, id: ConnectionId, request: DataRequest) {
        // calling unwrap on index here is fine, because only place this function is called is in
        // `Router::consume` method, when the status after reading from commit log of the same
        // filter as `request` is "done", that is, the subscriber has caught up. In other words,
        // there has been atleast 1 call to `native_readv` for the same filter, which means if
        // `native_readv` hasn't paniced, so this won't panic either.
        let data = self.native.get_mut(request.filter_idx).unwrap();
        data.waiters.register(id, request);
    }

    /// Cleanup a connection from all the waiters
    pub fn clean(&mut self, id: ConnectionId) -> Vec<DataRequest> {
        let mut inflight = Vec::new();
        for (_, data) in self.native.iter_mut() {
            inflight.append(&mut data.waiters.remove(id));
        }

        inflight
    }

    pub fn insert_to_retained_publishes(
        &mut self,
        publish: Publish,
        publish_properties: Option<PublishProperties>,
        topic: Topic,
    ) {
        let pub_with_props = (publish, publish_properties);
        self.retained_publishes.insert(topic, pub_with_props.into());
    }

    pub fn remove_from_retained_publishes(&mut self, topic: Topic) {
        self.retained_publishes.remove(&topic);
    }

    pub fn read_retained_messages(&mut self, filter: &str) -> Vec<PubWithProp> {
        trace!(info = "reading retain msg", filter = &filter);
        let now = Instant::now();

        // discard expired retained messages
        self.retained_publishes.retain(|_, pubdata| {
            // Keep data if no properties exists, which implies no message expiry!
            let Some(properties) = pubdata.properties.as_mut() else {
                return true;
            };

            // Keep data if there is no message_expiry_interval
            let Some(message_expiry_interval) = properties.message_expiry_interval.as_mut() else {
                return true;
            };

            let time_spent = (now - pubdata.timestamp).as_secs() as u32;

            let is_valid = time_spent < *message_expiry_interval;

            // ignore expired messages
            if is_valid {
                // set message_expiry_interval to (original value - time spent waiting in server)
                // ref: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901112
                *message_expiry_interval -= time_spent;
            }

            is_valid
        });

        // no need to include timestamp when returning
        self.retained_publishes
            .iter()
            .filter(|(topic, _)| matches(topic, filter))
            .map(|(_, p)| (p.publish.clone(), p.properties.clone()))
            .collect()
    }
}

pub struct Data<T> {
    filter: Filter,
    pub log: CommitLog<T>,
    pub waiters: Waiters<DataRequest>,
    meter: SubscriptionMeter,
}

impl<T> Data<T>
where
    T: Storage + Clone,
{
    pub fn new(filter: &str, router_config: &RouterConfig) -> Data<T> {
        let mut max_segment_size = router_config.max_segment_size;
        let mut max_mem_segments = router_config.max_segment_count;

        // Override segment config for selected filter
        if let Some(config) = &router_config.custom_segment {
            for (f, segment_config) in config {
                if matches(filter, f) {
                    info!("Overriding segment config for filter: {}", filter);
                    max_segment_size = segment_config.max_segment_size;
                    max_mem_segments = segment_config.max_segment_count;
                }
            }
        }

        // max_segment_size: usize, max_mem_segments: usize
        let log = CommitLog::new(max_segment_size, max_mem_segments).unwrap();

        let waiters = Waiters::with_capacity(10);
        let metrics = SubscriptionMeter::default();
        Data {
            filter: filter.to_owned(),
            log,
            waiters,
            meter: metrics,
        }
    }

    /// Writes to all the filters that are mapped to this publish topic
    /// and wakes up consumers that are matching this topic (if they exist)
    pub fn append(
        &mut self,
        item: T,
        notifications: &mut VecDeque<(ConnectionId, DataRequest)>,
    ) -> (Offset, &Filter) {
        let size = item.size();
        let offset = self.log.append(item);
        if let Some(mut parked) = self.waiters.take() {
            notifications.append(&mut parked);
        }

        self.meter.count += 1;
        self.meter.total_size += size;

        (offset, &self.filter)
    }
}

/// Acks log for a subscription
#[derive(Debug)]
pub struct AckLog {
    // Committed acks per connection. First pkid, last pkid, data
    committed: VecDeque<Ack>,
    // Recorded qos 2 publishes
    recorded: VecDeque<(Publish, Option<PublishProperties>)>,
}

impl AckLog {
    /// New log
    pub fn new() -> AckLog {
        AckLog {
            committed: VecDeque::with_capacity(100),
            recorded: VecDeque::with_capacity(100),
        }
    }

    pub fn connack(&mut self, id: ConnectionId, ack: ConnAck, props: Option<ConnAckProperties>) {
        let ack = Ack::ConnAck(id, ack, props);
        self.committed.push_back(ack);
    }

    pub fn suback(&mut self, ack: SubAck) {
        let ack = Ack::SubAck(ack);
        self.committed.push_back(ack);
    }

    pub fn puback(&mut self, ack: PubAck) {
        let ack = Ack::PubAck(ack);
        self.committed.push_back(ack);
    }

    pub fn pubrec(&mut self, publish: Publish, props: Option<PublishProperties>, ack: PubRec) {
        let ack = Ack::PubRec(ack);
        self.recorded.push_back((publish, props));
        self.committed.push_back(ack);
    }

    pub fn pubrel(&mut self, ack: PubRel) {
        let ack = Ack::PubRel(ack);
        self.committed.push_back(ack);
    }

    pub fn pubcomp(&mut self, ack: PubComp) -> Option<(Publish, Option<PublishProperties>)> {
        let ack = Ack::PubComp(ack);
        self.committed.push_back(ack);
        self.recorded.pop_front()
    }

    pub fn pingresp(&mut self, ack: PingResp) {
        let ack = Ack::PingResp(ack);
        self.committed.push_back(ack);
    }

    pub fn unsuback(&mut self, ack: UnsubAck) {
        let ack = Ack::UnsubAck(ack);
        self.committed.push_back(ack);
    }

    pub fn readv(&mut self) -> &mut VecDeque<Ack> {
        &mut self.committed
    }
}



/// There are 2 limits which are enforced:
/// - limit on size of each segment created by this log in bytes
/// - limit on number of segments in memory
///
/// When the active_segment is filled up, we move it to memory segments and empty it for new logs.
/// When the limit on the number of memory segments is reached, we remove the oldest segment from
/// memory segments.
///
/// This shifting of segments happens everytime the limit on the size of a segment exceeds the
/// limit. Note that the size of a segment might go beyond the limit if the single last log was put
/// at the offset which is within the limit but the logs size was large enough to be beyond the
/// limit. Only when another log is appended will we flush the active segment onto memory segments.
///
/// ### Invariants
/// - The active segment should have index `tail`.
/// - Segments throughout should be contiguous in their indices.
/// - The total size in bytes for each segment in memory should not increase beyond the
///   max_segment_size by more than the overflowing bytes of the last packet.
///
/// ### Seperation of implementation
///    - `index` & `segment` - everything directly related to files, no bounds check except when
///      bounds exceed file's existing size.
///    - `chunk` - abstraction to deal with index and segment combined. Basically we only need
///      stuff from segment file, and thus we hide away the index file under this abstraction.
///    - `segment` - abstracts away the memory segments for ease of access.
pub struct CommitLog<T> {
    /// The index at which segments start.
    head: u64,
    /// The index at which the current active segment is, and also marks the last valid segment as
    /// well as the last segment in memory.
    tail: u64,
    /// Maximum size of any segment in memory in bytes.
    max_segment_size: usize,
    /// Maximum number of segments in memory, apart from the active segment.
    max_mem_segments: usize,
    /// Total size of active segment, used for enforcing the contraints.
    segments: VecDeque<Segment<T>>,
}

impl<T> CommitLog<T>
where
    T: Storage + Clone,
{
    /// Create a new `CommitLog` with the given contraints. If `max_mem_segments` is 0, then only
    /// the active segment is maintained.
    pub fn new(max_segment_size: usize, max_mem_segments: usize) -> io::Result<Self> {
        if max_segment_size < 1024 {
            panic!("given max_segment_size {max_segment_size} bytes < 1KB");
        }

        if max_mem_segments < 1 {
            panic!("at least 1 segment needs to exist in memory else what's the point of log");
        }

        let mut segments = VecDeque::with_capacity(max_mem_segments);
        segments.push_back(Segment::new());

        Ok(Self {
            head: 0,
            tail: 0,
            max_segment_size,
            max_mem_segments,
            segments,
        })
    }

    #[inline]
    pub fn next_offset(&self) -> (u64, u64) {
        // `unwrap` fine as we are guaranteed that active segment always exist and is at the end
        (self.tail, self.active_segment().next_offset())
    }

    #[inline]
    pub fn _head_and_tail(&self) -> (u64, u64) {
        (self.head, self.tail)
    }

    #[inline]
    pub fn memory_segments_count(&self) -> usize {
        self.segments.len()
    }

    /// Size of data in all the segments
    #[allow(dead_code)]
    pub fn size(&self) -> u64 {
        let mut size = 0;
        for segment in self.segments.iter() {
            size += segment.size();
        }
        size
    }

    /// Number of segments
    #[allow(dead_code)]
    #[inline]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Number of packets
    #[inline]
    #[allow(dead_code)]
    pub fn entries(&self) -> u64 {
        self.active_segment().next_offset()
    }

    #[inline]
    fn active_segment(&self) -> &Segment<T> {
        self.segments.back().unwrap()
    }

    #[inline]
    fn active_segment_mut(&mut self) -> &mut Segment<T> {
        self.segments.back_mut().unwrap()
    }

    /// Append a new [`T`] to the active segment.
    #[inline]
    pub fn append(&mut self, message: T) -> (u64, u64) {
        self.apply_retention();
        let active_segment = self.active_segment_mut();
        active_segment.push(message);
        let absolute_offset = self.active_segment().next_offset();
        (self.tail, absolute_offset)
    }

    fn apply_retention(&mut self) {
        if self.active_segment().size() >= self.max_segment_size as u64 {
            // Read absolute_offset before applying memory retention, in case there is only 1
            // segment allowed.
            let absolute_offset = self.active_segment().next_offset();
            // If active segment is full and segments are full, apply retention policy.
            if self.memory_segments_count() >= self.max_mem_segments {
                self.segments.pop_front();
                self.head += 1;
            }

            // Pushing a new segment into segments and updating the tail automatically changes
            // the active segment to a new empty one.
            self.segments
                .push_back(Segment::with_offset(absolute_offset));
            self.tail += 1;
        }
    }

    #[inline]
    pub fn last(&self) -> Option<T> {
        self.active_segment().last()
    }

    /// Read `len` Ts at once. More efficient than reading 1 at a time. Returns
    /// the next offset to read data from. The Position::start returned need not
    /// be a valid index if the start given is not valid either.
    pub fn readv(
        &self,
        mut start: (u64, u64),
        mut len: u64,
        out: &mut Vec<(T, Offset)>,
    ) -> io::Result<Position> {
        let mut cursor = start;
        let _orig_cursor = cursor;

        if cursor.0 > self.tail {
            return Ok(Position::Done { start, end: start });
        }

        if cursor.0 < self.head {
            let head_absolute_offset = self.segments.front().unwrap().absolute_offset;
            warn!(
                "given index {} less than head {}, jumping to head",
                cursor.0, head_absolute_offset
            );
            cursor = (self.head, head_absolute_offset);
            start = cursor;
        }

        let mut idx = (cursor.0 - self.head) as usize;
        let mut curr_segment = &self.segments[idx];

        if curr_segment.absolute_offset > cursor.1 {
            warn!(
                "offset specified {} if less than actual {}, jumping",
                cursor.1, curr_segment.absolute_offset
            );
            start.1 = curr_segment.absolute_offset;
            cursor.1 = curr_segment.absolute_offset;
        }

        while cursor.0 < self.tail {
            // `Segment::readv` handles the conversion from absolute index to relative
            // index and it returns the absolute offset.
            // absolute cursor not to be confused with absolute offset
            match curr_segment.readv(cursor, len, out)? {
                // an offset returned -> we didn't read till end -> len fulfilled -> return
                SegmentPosition::Next(offset) => {
                    return Ok(Position::Next {
                        start,
                        end: (cursor.0, offset),
                    });
                }
                // no offset returned -> we reached end
                // if len unfulfilled -> try next segment with remaining length
                SegmentPosition::Done(next_offset) => {
                    // This condition is needed in case cursor.1 > 0 (when the user provies cursor.1
                    // beyond segment's last offset which can happen due to next readv offset
                    // being off by 1 before jumping to next segment or while manually reading
                    // from a particular cursor). In such case, the no. of read data points is
                    // 0 and hence we don't decrement len.
                    if next_offset >= cursor.1 {
                        len -= next_offset - cursor.1;
                    }
                    cursor = (cursor.0 + 1, next_offset);
                }
            }

            if len == 0 {
                // debug!("start: {:?}, end: ({}, {})", orig_cursor, cursor.0, cursor.1 - 1);
                return Ok(Position::Next { start, end: cursor });
            }

            idx += 1;
            curr_segment = &self.segments[idx];
        }

        if curr_segment.next_offset() <= cursor.1 {
            return Ok(Position::Done { start, end: cursor });
        }

        // We need to read seperately from active segment because if `None` is returned for active
        // segment's `readv`, then we should return `None` as well as it is not possible to read
        // further, whereas for older segments we simply jump on to the new one to read more.

        match curr_segment.readv(cursor, len, out)? {
            SegmentPosition::Next(v) => {
                // debug!("start: {:?}, end: ({}, {})", orig_cursor, cursor.0, cursor.1 + v - 1);
                Ok(Position::Next {
                    start,
                    end: (cursor.0, v),
                })
            }
            SegmentPosition::Done(absolute_offset) => {
                // debug!("start: {:?}, end: ({}, {}) done", orig_cursor, cursor.0, absolute_offset);
                Ok(Position::Done {
                    start,
                    end: (cursor.0, absolute_offset),
                })
            }
        }
    }
}

pub(crate) struct Segment<T> {
    /// Holds the actual segment.
    pub(crate) data: Vec<T>,
    total_size: u64,
    /// The absolute offset at which the `inner` starts at. All reads will return the absolute
    /// offset as the offset of the cursor.
    ///
    /// **NOTE**: this offset is re-generated on each run of the commit log.
    pub(crate) absolute_offset: u64,
}

pub(crate) enum SegmentPosition {
    /// When the returned absolute offset exists within the current segment
    Next(u64),
    /// When the the returned absolute offset does not exist within the current segment, but
    /// instead is 1 beyond the highest absolute offset in this segment, meant for use with next
    /// segment if any exists.
    Done(u64),
}

impl<T> Segment<T>
where
    T: Storage + Clone,
{
    pub(crate) fn with_offset(absolute_offset: u64) -> Self {
        Self {
            data: Vec::with_capacity(1024),
            absolute_offset,
            total_size: 0,
        }
    }
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::with_capacity(1024),
            absolute_offset: 0,
            total_size: 0,
        }
    }

    #[inline]
    pub(crate) fn next_offset(&self) -> u64 {
        self.absolute_offset + self.len()
    }

    /// Push a new `T` in the segment.
    #[inline]
    pub(crate) fn push(&mut self, inner_type: T) {
        self.total_size += inner_type.size() as u64;
        self.data.push(inner_type);
    }

    #[inline]
    /// Takes in the abosolute index to start reading from. Internally handles the conversion from
    /// relative offset to absolute offset and vice-versa.
    pub(crate) fn readv(
        &self,
        cursor: Cursor,
        len: u64,
        out: &mut Vec<(T, Offset)>,
    ) -> io::Result<SegmentPosition> {
        // This substraction can never overflow as checking of offset happens at
        // `CommitLog::readv`.
        let idx = cursor.1 - self.absolute_offset;

        let mut ret: Option<u64>;

        if idx >= self.len() {
            ret = None;
        } else {
            let mut limit = idx + len;

            ret = Some(limit);

            if limit >= self.len() {
                ret = None;
                limit = self.len();
            }
            let offsets = std::iter::repeat(cursor.0).zip(cursor.1..cursor.1 + limit);
            let o = self.data[idx as usize..limit as usize]
                .iter()
                .cloned()
                .zip(offsets);
            out.extend(o);
        }

        match ret {
            Some(relative_offset) => Ok(SegmentPosition::Next(
                self.absolute_offset + relative_offset,
            )),
            None => Ok(SegmentPosition::Done(self.next_offset())),
        }
    }

    /// Get the number of `T` in the segment.
    #[inline]
    pub(crate) fn len(&self) -> u64 {
        self.data.len() as u64
    }

    /// Get the total size in bytes of the segment.
    #[inline]
    pub(crate) fn size(&self) -> u64 {
        self.total_size
    }

    #[inline]
    pub fn last(&self) -> Option<T> {
        self.data.last().cloned()
    }
}

