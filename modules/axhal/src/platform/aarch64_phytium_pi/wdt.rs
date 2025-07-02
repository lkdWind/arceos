use aarch64_cpu::registers::{Readable, Writeable};
use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

register_bitfields![u32,
    pub WDT_WRR [
        FEED_DOG OFFSET(0) NUMBITS(32) [
            clear = 0,
            set = 1,
        ],
    ]
];

register_bitfields![u32,
    pub WDT_W_IIDR [
        JEP_ID OFFSET(0) NUMBITS(7) [
            clear = 0,
            set = 1
        ],
        JEP_CONT OFFSET(8) NUMBITS(4) [
            clear = 0,
            set = 1
        ],
        VERSION OFFSET(16) NUMBITS(4) [
            clear = 0,
            set = 1 
        ],
    ]
];

register_bitfields![u32,
    pub WDT_WCS [
        ENABLE OFFSET(0) NUMBITS(1) [
            clear = 0,
            set = 1,
        ],
        WS0 OFFSET(1) NUMBITS(1) [
            clear = 0,
            set = 1,
        ],
        WS1 OFFSET(2) NUMBITS(1) [
            clear = 0,
            set = 1,
        ],
    ]
];

register_bitfields![u32,
    pub WDT_WOR [
        TIMEOUT OFFSET(0) NUMBITS(32)[
            clear = 0,
            set = 1
        ]
    ]
];

register_bitfields![u32,
    pub WDT_WCVL [
        wcvl OFFSET(0) NUMBITS(32)[
            clear = 0,
            set = 1
        ]
    ]
];

register_bitfields![u32,
    pub WDT_WCVH [
        wcvh OFFSET(0) NUMBITS(32)[
            clear = 0,
            set = 1,
        ]
    ]
];

register_structs! {
    pub WDTRegs {
        (0x0000 => pub wrr: ReadWrite<u32, WDT_WRR::Register>),
        (0x0004 => __reserved0),
        (0x0fcc => pub wiidr: ReadOnly<u32, WDT_W_IIDR::Register>),
        (0x0fd0 => __reserved1),
        (0x1000 => pub wcs: ReadWrite<u32, WDT_WCS::Register>),
        (0x1004 => __reserved2),
        (0x1008 => pub wor: ReadWrite<u32, WDT_WOR::Register>),
        (0x100c => __reserved3),
        (0x1010 => pub wcvl: ReadWrite<u32, WDT_WCVL::Register>),
        (0x1014 => pub wcvh: ReadWrite<u32, WDT_WCVH::Register>),
        (0x1018 => @END),
    }
}

pub const WDT_REGS: *mut WDTRegs =
    (axconfig::plat::PHYS_VIRT_OFFSET + 0x2804_0000) as *mut WDTRegs;

pub fn wdt_init() {
    use crate::irq::register_handler;
    unsafe {
        let wdt0: &WDTRegs = &mut *WDT_REGS;
        let version = wdt0.wiidr.read(WDT_W_IIDR::VERSION) as u8;
        let id = wdt0.wiidr.read(WDT_W_IIDR::JEP_ID) as u8;
        info!("WDT VERSION: {}, ID: {}", version, id)
    }

    info!("Initializing WatchDog interrupt");
    const WDT0_IRQ_NUM: usize = 196;
    const WDT1_IRQ_NUM: usize = 197;
    // set interrupt enable
    crate::irq::set_enable(WDT0_IRQ_NUM, true);
    // crate::irq::set_enable(WDT1_IRQ_NUM, true);
    // register handler
    
    register_handler(WDT0_IRQ_NUM, handle_wdt0);
    // register_handler(WDT1_IRQ_NUM, handle_wdt1);

    unsafe {
        let wdt: &WDTRegs = &mut *WDT_REGS;
        // set timeout
        const TIME: u32 = 3000000 * 2;
        wdt.wor.write(WDT_WOR::TIMEOUT.val(TIME));
        info!("set WDT WOR::TIMEOUT {}", wdt.wor.read(WDT_WOR::TIMEOUT));
        // enable
        wdt.wcs.write(WDT_WCS::ENABLE.val(1));
        info!("set WDT WCS::ENABLE {}", wdt.wcs.read(WDT_WCS::ENABLE));

        info!("start get wcs loop");
        loop {
            info!("now wcs: {:x}", wdt.wcs.get());
        }
    }
}

fn handle_wdt0() {
    info!("wdt0 interrupt trigger");
    unsafe {
        let wdt0: &WDTRegs = &mut *WDT_REGS;
        wdt0.wrr.write(WDT_WRR::FEED_DOG.val(0x1));
    }
}