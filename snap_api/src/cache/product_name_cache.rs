use common_define::db::SnapProductInfoModel;
use common_define::Id;
use derive_new::new;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, new)]
pub struct ProductNameCache {
    pub v: Mutex<HashMap<Id, SnapProductInfoModel>>,
    c: Mutex<Vec<SnapProductInfoModel>>,
}

impl Default for ProductNameCache {
    fn default() -> Self {
        ProductNameCache::new(Mutex::new(HashMap::new()), Mutex::new(Vec::new()))
    }
}

impl ProductNameCache {
    pub fn delete_by_product_id(&self, id: &Id) {
        self.v.lock().unwrap().remove(id);
    }
    pub fn replace(&self, products: Vec<SnapProductInfoModel>) {
        let mut map = self.v.lock().unwrap();
        for product in products.iter() {
            map.insert(product.id, product.clone());
        }
        let mut c = self.c.lock().unwrap();
        c.clear();
        c.extend(products);
    }
    pub fn insert(&self, product: SnapProductInfoModel) {
        self.v.lock().unwrap().insert(product.id, product.clone());
        self.c.lock().unwrap().push(product);
    }
    pub fn get_by_id(&self, product_id: Option<Id>) -> Option<SnapProductInfoModel> {
        let g = self.v.lock().unwrap();
        match product_id {
            Some(id) => match g.get(&id) {
                Some(g) => Some(g.clone()),
                None => g.get(&Id::new(1)).cloned(),
            },
            None => g.get(&Id::new(1)).cloned(),
        }
    }
    pub fn get_all_product(&self) -> Vec<SnapProductInfoModel> {
        let c = self.c.lock().unwrap();
        c.clone()
    }
}
