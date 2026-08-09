use kalcite_engine_core::{Canvas,Color565,Rect};
use kalcite_platform_api::{Buttons,Platform};
use kalcite_platform_headless::Headless;

const W:u16=320; const H:u16=240; const PIXELS:usize=W as usize*H as usize;
#[derive(Default)]struct Ball{x:i16,y:i16,vx:i16,vy:i16}
fn main(){
    let mut platform:Headless<PIXELS>=Headless::new(W,H);
    let mut framebuffer=[0u16;PIXELS];
    let mut ball=Ball{x:156,y:116,vx:2,vy:1}; let mut paddle=100i16;
    for frame in 0..180{
        platform.now=frame*16;
        platform.input=if frame%80<40{Buttons(Buttons::UP)}else{Buttons(Buttons::DOWN)};
        let input=platform.buttons();
        if input.held(Buttons::UP){paddle=(paddle-2).max(0)}
        if input.held(Buttons::DOWN){paddle=(paddle+2).min(H as i16-40)}
        ball.x+=ball.vx;ball.y+=ball.vy;
        if ball.y<=0||ball.y>=H as i16-8{ball.vy=-ball.vy}
        if ball.x<=8&&ball.y+8>=paddle&&ball.y<=paddle+40{ball.vx=ball.vx.abs()}
        if ball.x>=W as i16-8{ball.vx=-ball.vx.abs()}
        if ball.x<0{ball.x=156;ball.y=116;ball.vx=2}
        let mut c=Canvas::new(&mut framebuffer,W,H).unwrap();c.clear(Color565::BLACK);
        c.rect(Rect{x:4,y:paddle,w:4,h:40},Color565::WHITE);
        c.rect(Rect{x:ball.x,y:ball.y,w:8,h:8},Color565::WHITE);
        platform.present(c.pixels());
    }
    println!("Pong headless: {} frames, checksum {}",platform.presents,platform.frame.iter().fold(0u32,|a,&x|a.wrapping_add(x as u32)));
}
