pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod joypad;
pub mod opcodes;
pub mod ppu;
pub mod render;
pub mod trace;

use bus::Bus;
use cartridge::Rom;
use cpu::CPU;
use ppu::NesPPU;
use render::frame::Frame;
use trace::trace;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

fn main() {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("PAC MAN", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build().unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240).unwrap();
    
    let bytes: Vec<u8> = std::fs::read("snake.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let mut frame = Frame::new();

    let mut key_map1 = HashMap::new();
    key_map1.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    key_map1.insert(Keycode::Up, joypad::JoypadButton::UP);
    key_map1.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    key_map1.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    key_map1.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    key_map1.insert(Keycode::Return, joypad::JoypadButton::START);
    key_map1.insert(Keycode::K, joypad::JoypadButton::BUTTON_A);
    key_map1.insert(Keycode::L, joypad::JoypadButton::BUTTON_B);

    let mut key_map2 = HashMap::new();
    key_map2.insert(Keycode::S, joypad::JoypadButton::DOWN);
    key_map2.insert(Keycode::W, joypad::JoypadButton::UP);
    key_map2.insert(Keycode::D, joypad::JoypadButton::RIGHT);
    key_map2.insert(Keycode::A, joypad::JoypadButton::LEFT);
    key_map2.insert(Keycode::C, joypad::JoypadButton::SELECT);
    key_map2.insert(Keycode::V, joypad::JoypadButton::START);
    key_map2.insert(Keycode::N, joypad::JoypadButton::BUTTON_A);
    key_map2.insert(Keycode::M, joypad::JoypadButton::BUTTON_B);

    let bus = Bus::new(rom, move |ppu: &NesPPU, joypad1: &mut joypad::Joypad, joypad2: &mut joypad::Joypad| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),


                Event::KeyDown { keycode, .. } => {
                    if let Some(keycode) = keycode {
                        if let Some(key) = key_map1.get(&keycode) {
                            joypad1.set_button_pressed_status(*key, true);
                        }
                        if let Some(key) = key_map2.get(&keycode) {
                            joypad2.set_button_pressed_status(*key, true);
                        }
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(keycode) = keycode {
                        if let Some(key) = key_map1.get(&keycode) {
                            joypad1.set_button_pressed_status(*key, false);
                        }
                        if let Some(key) = key_map2.get(&keycode) {
                            joypad2.set_button_pressed_status(*key, false);
                        }
                    }
                }

                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();

}