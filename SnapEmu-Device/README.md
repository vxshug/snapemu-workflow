# SnapEmu-Device: SnapEmu Device Library

## How to add?

```txt
/device/Manufacturers Directory/Device Directory/
```
Representing a single device using a json file, 

Example: `AB01.json`

```json
{
  "name": "AB01",
  "picture": "ab01.webp",
  "description": "HTCC-AB01 (V2) is based on ASR6052, the chip is already integrated with the PSoC® 4000 series MCU (ARM® Cortex® M0+ Core) and SX1262.",
  "demo": "https://docs.heltec.org/en/node/asr650x/htcc_ab01/index.html",
  "manufacturer": "Heltec Automation",
  "type": "Node",
  "protocol": "LoRaWAN"
}
```

- name: Device Name
- picture: Relative path to the device image
- description: Device Description
- demo: Example Tutorial
- manufacturer: Manufacturer Name
- type: Device type, Node or Gateway
- prorocol: Device protocol, LoRaWAN or MQTT
