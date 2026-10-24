// Infrastructure calculator only; no Halcyon or COSMIC binary is built.
// GeoPosition encoding is copied verbatim from the pinned native geonames lib.
use std::collections::BTreeMap;
use chrono::NaiveDate;
use sunrise::{Coordinates, SolarDay, SolarEvent};
#[derive(Clone, Debug, bitcode::Decode, bitcode::Encode)]
pub struct GeoPosition { pub latitude: f64, pub longitude: f64 }
fn main() {
 let data = include_bytes!("../../source/data-timezone-geodata.bitcode-v0-6");
 let map: BTreeMap<String,GeoPosition> = bitcode::decode(data).unwrap();
 let zone="Europe/London";
 let p=map.get(zone).unwrap();
 let day=SolarDay::new(Coordinates::new(p.latitude,p.longitude).unwrap(),NaiveDate::from_ymd_opt(2026,10,23).unwrap());
 println!("{{\"native_source\":\"655b14fb9572dd7f7231ec6c343c63984e99741a\",\"timezone\":\"{}\",\"latitude\":{},\"longitude\":{},\"sunrise_utc\":\"{}\",\"sunrise_epoch\":{},\"sunset_utc\":\"{}\",\"sunset_epoch\":{},\"sunrise_crate\":\"2.1.0\",\"bitcode_crate\":\"0.6.9\",\"chrono_crate\":\"0.4.45\"}}",zone,p.latitude,p.longitude,day.event_time(SolarEvent::Sunrise).to_rfc3339(),day.event_time(SolarEvent::Sunrise).timestamp(),day.event_time(SolarEvent::Sunset).to_rfc3339(),day.event_time(SolarEvent::Sunset).timestamp());
}
