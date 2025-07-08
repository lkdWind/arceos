#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;
use axstd::console::phyt_pl011;

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    println!("Phytium pl011 test start");
    unsafe {
        let mut pl011 = phyt_pl011::Pl011::new(0x2800_E000);
        pl011.init();
        for i in 0..10 {
            pl011.send_byte(i);
            println!("Send byte: {}\n", i);
            let recv = pl011.recv_byte();
            println!("Recv byte: {}\n", recv);
        }
    }
    println!("Phytium pl011 test end");
}
