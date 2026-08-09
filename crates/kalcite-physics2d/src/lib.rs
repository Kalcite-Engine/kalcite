#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub struct Aabb{pub x:i32,pub y:i32,pub w:i32,pub h:i32}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)] pub struct Motion{pub dx:i32,pub dy:i32}
pub fn hit(a:Aabb,b:Aabb)->bool{a.x<b.x+b.w&&a.x+a.w>b.x&&a.y<b.y+b.h&&a.y+a.h>b.y}
pub fn move_and_slide(mut body:Aabb, motion:Motion, solids:&[Aabb])->(Aabb,Motion){let mut applied=Motion::default();let next=Aabb{x:body.x+motion.dx,..body};if !solids.iter().copied().any(|s|hit(next,s)){body.x=next.x;applied.dx=motion.dx;}let next=Aabb{y:body.y+motion.dy,..body};if !solids.iter().copied().any(|s|hit(next,s)){body.y=next.y;applied.dy=motion.dy;}(body,applied)}
#[cfg(test)]mod tests{use super::*;#[test]fn blocks_motion(){let b=Aabb{x:0,y:0,w:8,h:8};let wall=Aabb{x:8,y:0,w:8,h:8};let(n,m)=move_and_slide(b,Motion{dx:2,dy:3},&[wall]);assert_eq!(n.x,0);assert_eq!(m.dx,0);assert_eq!(n.y,3);}}
