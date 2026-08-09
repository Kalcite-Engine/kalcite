#[derive(Clone,Copy)]pub struct Aabb{pub x:i32,pub y:i32,pub w:i32,pub h:i32} pub fn hit(a:Aabb,b:Aabb)->bool{a.x<b.x+b.w&&a.x+a.w>b.x&&a.y<b.y+b.h&&a.y+a.h>b.y}
