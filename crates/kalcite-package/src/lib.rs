use std::collections::BTreeMap;#[derive(Default)]pub struct Lock{pub version:u32,pub packages:BTreeMap<String,Package>}pub struct Package{pub source:String,pub revision:String,pub checksum:String}
