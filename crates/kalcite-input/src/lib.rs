use std::collections::BTreeMap;
#[derive(Default,Clone,Copy,Debug)]struct State{now:bool,prev:bool}
#[derive(Default,Clone,Debug)]pub struct Actions{bindings:BTreeMap<String,Vec<u16>>,states:BTreeMap<String,State>}
impl Actions{
 pub fn bind(&mut self,a:&str,k:u16){let v=self.bindings.entry(a.into()).or_default();if !v.contains(&k){v.push(k)}}
 pub fn update(&mut self,f:impl Fn(u16)->bool){for(a,k)in&self.bindings{let s=self.states.entry(a.clone()).or_default();s.prev=s.now;s.now=k.iter().any(|x|f(*x))}}
 pub fn held(&self,a:&str)->bool{self.states.get(a).is_some_and(|x|x.now)} pub fn pressed(&self,a:&str)->bool{self.states.get(a).is_some_and(|x|x.now&&!x.prev)} pub fn released(&self,a:&str)->bool{self.states.get(a).is_some_and(|x|!x.now&&x.prev)}
 pub fn axis(&self,negative:&str,positive:&str)->i8{self.held(positive) as i8-self.held(negative) as i8}
 pub fn actions(&self)->impl Iterator<Item=&str>{self.bindings.keys().map(String::as_str)}
}
pub fn key_code(name:&str)->Option<u16>{Some(match name.trim(){"Left"=>1,"Right"=>2,"Up"=>3,"Down"=>4,"OK"=>5,"Back"=>6,"Home"=>7,"Plus"=>8,"Minus"=>9,_=>return None})}
pub fn parse_map(text:&str)->Result<Actions,String>{let mut a=Actions::default();for (n,raw) in text.lines().enumerate(){let line=raw.split('#').next().unwrap_or("").trim();if line.is_empty(){continue}let(name,keys)=line.split_once('=').ok_or_else(||format!("input line {}: expected Action=Key",n+1))?;for key in keys.split('|'){a.bind(name.trim(),key_code(key).ok_or_else(||format!("input line {}: unknown key `{}`",n+1,key.trim()))?)} }Ok(a)}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]pub struct Touch{pub id:u8,pub x:i16,pub y:i16,pub down:bool}
#[derive(Clone,Debug,Default)]pub struct Touches{items:[Touch;4]}
impl Touches{pub fn set(&mut self,index:usize,t:Touch){if let Some(v)=self.items.get_mut(index){*v=t}}pub fn active(&self)->impl Iterator<Item=&Touch>{self.items.iter().filter(|t|t.down)}}
#[cfg(test)]mod tests{use super::*;#[test]fn map(){let a=parse_map("Jump=OK\nMove=Left|Right").unwrap();assert_eq!(a.actions().count(),2)}#[test]fn edges(){let mut a=parse_map("Jump=OK").unwrap();a.update(|k|k==5);assert!(a.pressed("Jump"));a.update(|_|false);assert!(a.released("Jump"));}}
