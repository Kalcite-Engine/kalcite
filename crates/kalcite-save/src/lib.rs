pub struct Header{pub version:u32,pub schema:u32}pub trait Migratable:Sized{const VERSION:u32;fn migrate(from:u32,data:&[u8])->Result<Self,String>;}
