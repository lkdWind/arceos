use aarch64_cpu::registers::{Readable, Writeable};
use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

const I2C_BASE: usize = 0x2800_A000;
pub const I2C_REGS: &'static I2cRegs = unsafe { &*(I2C_BASE as *const I2cRegs) };
const I2C_TIMEOUT: u32 = 50000;

const I2C_DATA_MASK: u32 = gen_mask(7, 0);

fn div_round_up(n: u32, d: u32) -> u32 {
    assert!(d != 0, "Division by zero");
    (n + d - 1) / d
}

fn gen_mask(h: u32, l: u32) -> u32 {
    const BITS_PER_LONG: u32 = 32;
    ((!0u32).wrapping_sub(1u32 << l).wrapping_add(1)) & 
    (!0u32 >> (BITS_PER_LONG - 1 - h))
}

register_bitfields![u32,
    pub IC_CON [
        MASTER_MODE OFFSET(0) NUMBITS(1) [
            ENABLE = 1,
            DISABLE = 0,
        ],
        SPEED OFFSET(1) NUMBITS(2) [
            STANDARD = 1,   // standard speed <= 100Kbit/s
            QUICK = 2,      // quick speed <= 400Kbit/s
            HIGH = 3,       // high-speed <= 3.4Mbit/s
        ],
        IC_10BITADDR_SLAVE OFFSET(3) NUMBITS(1) [
            SEVEN_BITS_ADDR_MODE = 0,
            TEN_BITS_ADDR_MODE = 1,
        ],
        IC_10BITADDR_MASTER OFFSET(4) NUMBITS(1) [
            SEVEN_BITS_ADDR_MODE = 0,
            TEN_BITS_ADDR_MODE = 1,
        ],
        IC_RESTART_EN OFFSET(5) NUMBITS(1) [
            ENABLE = 1,
            DISABLE = 0,
        ],
        IC_SLAVE_DISABLE OFFSET(6) NUMBITS(1) [
            ENABLE = 1,
            DISABLE = 0,
        ],
    ]
];

register_bitfields![u32,
    pub IC_TAR [
        IC_TAR OFFSET(0) NUMBITS(10) [],
        GC_OR_START OFFSET(10) NUMBITS(1) [
            START_BYTE = 1,
            BROADCAST_CALL = 0,
        ],
        SPECIAL OFFSET(11) NUMBITS(1) [
            GC_OR_START_MODE = 1,
            IC_TAR_MODE = 0,
        ],
        IC_10BITADDR_MASTER OFFSET(12) NUMBITS(1) [
            TEN_BITS_ADDR_MODE = 1,
            SEVEN_BITS_ADDR_MODE = 0,
        ],
    ]
];

register_bitfields![u32,
    pub IC_SAR [
        IC_SAR OFFSET(0) NUMBITS(10) []
    ]
];

register_bitfields![u32,
    pub IC_HS_MADDR [
        IC_HS_MAD OFFSET(0) NUMBITS(3) []
    ]
];

register_bitfields![u32,
    pub IC_DATA_CMD [
        DAT OFFSET(0) NUMBITS(8) [],
        CMD OFFSET(8) NUMBITS(1) [
            MASTER_READ_MODE = 1,
            MASTER_WRITE_MODE = 0,
        ],
        STOP OFFSET(9) NUMBITS(1) [
            ENABLE = 1,
            DISABLE = 0,
        ],
        RESTART OFFSET(10) NUMBITS(1) [
            ENABLE = 1,
            DISABLE = 0,
        ],
    ]
];

register_bitfields![u32,
    pub IC_SS_SCL_HCNT [
        IC_SS_SCL_HCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_SS_SCL_LCNT [
        IC_SS_SCL_LCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_FS_SCL_HCNT [
        IC_FS_SCL_HCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_FS_SCL_LCNT [
        IC_FS_SCL_LCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_HS_SCL_HCNT [
        IC_HS_SCL_HCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_HS_SCL_LCNT [
        IC_HS_SCL_LCNT OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_INTR_STAT [
        R_RX_UNDER OFFSET(0) NUMBITS(1) [],
        R_RX_OVER OFFSET(1) NUMBITS(1) [],
        R_RX_FULL OFFSET(2) NUMBITS(1) [],
        R_TX_OVER OFFSET(3) NUMBITS(1) [],
        R_TX_EMPTY OFFSET(4) NUMBITS(1) [],
        R_RD_REQ OFFSET(5) NUMBITS(1) [],
        R_TX_ABRT OFFSET(6) NUMBITS(1) [],
        R_RX_DONE OFFSET(7) NUMBITS(1) [],
        R_ACTIVITY OFFSET(8) NUMBITS(1) [],
        R_STOP_DET OFFSET(9) NUMBITS(1) [],
        R_START_DET OFFSET(10) NUMBITS(1) [],
        R_GEN_CALL OFFSET(11) NUMBITS(1) [],
    ]
];

register_bitfields![u32,
    pub IC_INTR_MASK [
        M_RX_UNDER OFFSET(0) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_RX_OVER OFFSET(1) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_RX_FULL OFFSET(2) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_TX_OVER OFFSET(3) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_TX_EMPTY OFFSET(4) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_RD_REQ OFFSET(5) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_TX_ABRT OFFSET(6) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_RX_DONE OFFSET(7) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_ACTIVITY OFFSET(8) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_STOP_DET OFFSET(9) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_START_DET OFFSET(10) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
        M_GEN_CALL OFFSET(11) NUMBITS(1) [
            CLEAR = 0,
            SET = 1,
        ],
    ]
];

register_bitfields![u32,
    pub IC_RAW_INTR_STAT [
        RX_UNDER OFFSET(0) NUMBITS(1) [],
        RX_OVER OFFSET(1) NUMBITS(1) [],
        RX_FULL OFFSET(2) NUMBITS(1) [],
        TX_OVER OFFSET(3) NUMBITS(1) [],
        TX_EMPTY OFFSET(4) NUMBITS(1) [],
        RD_REQ OFFSET(5) NUMBITS(1) [],
        TX_ABRT OFFSET(6) NUMBITS(1) [],
        RX_DONE OFFSET(7) NUMBITS(1) [],
        ACTIVITY OFFSET(8) NUMBITS(1) [],
        STOP_DET OFFSET(9) NUMBITS(1) [],
        START_DET OFFSET(10) NUMBITS(1) [],
        GEN_CALL OFFSET(11) NUMBITS(1) [],
    ]
];

register_bitfields![u32,
    pub IC_RX_TL [
        RX_TL OFFSET(0) NUMBITS(8) []
    ]
];

register_bitfields![u32,
    pub IC_TX_TL [
        TX_TL OFFSET(0) NUMBITS(8) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_INTR [
        CLR_INTR OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_RX_UNDER [
        CLR_RX_UNDER OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_RX_OVER [
        CLR_RX_OVER OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_TX_OVER [
        CLR_TX_OVER OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_RD_REQ [
        CLR_RD_REQ OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_TX_ABRT [
        CLR_TX_ABRT OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_RX_DONE [
        CLR_RX_DONE OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_ACTIVITY [
        CLR_ACTIVITY OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_STOP_DET [
        CLR_STOP_DET OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_START_DET [
        CLR_START_DET OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_CLR_GEN_CALL [
        CLR_GEN_CALL OFFSET(0) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_ENABLE [
        ENABLE OFFSET(0) NUMBITS(1) [
            OFF_CONTROLLER = 0,
            ON_CONTROLLER = 1,
        ]
    ]
];

register_bitfields![u32,
    pub IC_STATUS [
        ACTIVITY OFFSET(0) NUMBITS(1) [
            // Indicates the controller is actively transmitting data,
            // valid in both master and slave modes.
            TRANSMITTING_DATA = 1,
            // Indicates the controller is in idle state.
            IDLE = 0,
        ],
        // Set when one or more locations in the transmit FIFO are empty;
        // cleared when the transmit FIFO is full.
        TFNF OFFSET(1) NUMBITS(1) [
            // Transmit FIFO full
            FIFO_FULL = 0,
            // Transmit FIFO not full
            FIFO_NOT_FULL = 1,
        ],
        // Set when the transmit FIFO is completely empty;
        // cleared when one or more entries exist in the FIFO. 
        // This flag does not generate an interrupt.
        TFE OFFSET(2) NUMBITS(1) [
            SEND_FIFO_EMPTY = 1,
            SEND_FIFO_NOT_EMPTY = 0,
        ],
        RFNE OFFSET(3) NUMBITS(1) [
            RECV_FIFO_EMPTY = 0,
            RECV_FIFO_NOT_EMPTY = 1,
        ],
        RFF OFFSET(4) NUMBITS(1) [
            RECV_FIFO_FULL = 1,
            RECV_FIFO_NOT_FULL = 0,
        ],
        MST_ACTIVITY OFFSET(5) NUMBITS(1) [
            MASTER_FSM_IDLE = 0,
            MASTER_FSM_NOT_IDLE = 1,
        ],
        SLV_ACTIVITY OFFSET(6) NUMBITS(1) [
            SLAVE_FSM_IDLE = 0,
            SLAVE_FSM_NOT_IDLE = 1,
        ],
    ]
];

register_bitfields![u32,
    pub IC_TXFLR [
        TXFLR OFFSET(0) NUMBITS(3) []
    ]
];

register_bitfields![u32,
    pub IC_RXFLR [
        RXFLR OFFSET(0) NUMBITS(3) []
    ]
];

register_bitfields![u32,
    pub IC_SDA_HOLD [
        IC_SDA_HOLD OFFSET(0) NUMBITS(16) []
    ]
];

register_bitfields![u32,
    pub IC_TX_ABRT_SOURCE [
        ABRT_7B_ADDR_NOACK OFFSET(0) NUMBITS(1) [],
        ABRT_10ADDR1_NOACK OFFSET(1) NUMBITS(1) [],
        ABRT_10ADDR2_NOACK OFFSET(2) NUMBITS(1) [],
        ABRT_TXDATA_NOACK OFFSET(3) NUMBITS(1) [],
        ABRT_GCALL_NOACK OFFSET(4) NUMBITS(1) [],
        ABRT_GCALL_READ OFFSET(5) NUMBITS(1) [],
        ABRT_HS_ACKDET OFFSET(6) NUMBITS(1) [],
        ABRT_SBYTE_ACKDET OFFSET(7) NUMBITS(1) [],
        ABRT_HS_NORSTRT OFFSET(8) NUMBITS(1) [],
        ABRT_SBYTE_NORSTRT OFFSET(9) NUMBITS(1) [],
        ABRT_10B_RD_NORSTRT OFFSET(10) NUMBITS(1) [],
        ABRT_MASTER_DIS OFFSET(11) NUMBITS(1) [],
        ABRT_LOST OFFSET(12) NUMBITS(1) [],
        ABRT_SLVFLUSH_TXFIFO OFFSET(13) NUMBITS(1) [],
        ABRT_SLV_ARBLOST OFFSET(14) NUMBITS(1) [],
        ABRT_SLVRD_INTX OFFSET(15) NUMBITS(1) [],
    ]
];

register_bitfields![u32,
    pub IC_SLV_DATA_NACK_ONLY [
        NACK OFFSET(0) NUMBITS(1) [
            GENERATE_NACK = 1,
            GENERATE_NORMAL_CONDITIONAL_NACK = 0,
        ]
    ]
];

register_bitfields![u32,
    pub IC_DMA_CR [
        RDMAE OFFSET(0) NUMBITS(1) [
            SEND_DMA_DISBALE = 0,
            SEND_DMA_ENBALE = 1,
        ],
        TDMAE OFFSET(1) NUMBITS(1) [
            RECV_DMA_DISBALE = 0,
            RECV_DMA_ENBALE = 1,
        ]
    ]
];

register_bitfields![u32,
    pub IC_DMA_TDLR [
        DMA_TDL OFFSET(0) NUMBITS(2) []
    ]
];

register_bitfields![u32,
    pub IC_DMA_RDLR [
        DMA_RDL OFFSET(0) NUMBITS(2) []
    ]
];

register_bitfields![u32,
    pub IC_SDA_SETUP [
        SDA_SETUP OFFSET(0) NUMBITS(8) []
    ]
];

register_bitfields![u32,
    pub IC_ACK_GENERAL_CALL [
        ACK_GENERAL_CALL OFFSET(0) NUMBITS(1)
    ]
];

register_bitfields![u32,
    pub IC_ENABLE_STATUS [
        IC_EN OFFSET(0) NUMBITS(1) [
            CONTROLLER_CLOSED = 0,
            CONTROLLER_OPENED = 1,
        ],
        SLV_DISABLE_WHITE_BUSY OFFSET(1) NUMBITS(1) [],
        SLV_RX_DATA_LOST OFFSET(2) NUMBITS(1) []
    ]
];

register_bitfields![u32,
    pub IC_FS_SPKLEN [
        IC_FS_SPKLEN OFFSET(0) NUMBITS(8) []
    ]
];

register_bitfields![u32,
    pub IC_HS_SPKLEN [
        IC_HS_SPKLEN OFFSET(0) NUMBITS(8) []
    ]
];

register_structs! {
    pub I2cRegs {
        (0x00 => pub ic_con: ReadWrite<u32, IC_CON::Register>),
        (0x04 => pub ic_tar: ReadWrite<u32, IC_TAR::Register>),
        (0x08 => pub ic_sar: ReadWrite<u32, IC_SAR::Register>),
        (0x0C => pub ic_hs_maddr: ReadWrite<u32, IC_HS_MADDR::Register>),
        (0x10 => pub ic_data_cmd: WriteOnly<u32, IC_DATA_CMD::Register>),
        (0x14 => pub ic_ss_scl_hcnt: ReadWrite<u32, IC_SS_SCL_HCNT::Register>),
        (0x18 => pub ic_ss_scl_lcnt: ReadWrite<u32, IC_SS_SCL_LCNT::Register>),
        (0x1C => pub ic_fs_scl_hcnt: ReadWrite<u32, IC_FS_SCL_HCNT::Register>),
        (0x20 => pub ic_fs_scl_lcnt: ReadWrite<u32, IC_FS_SCL_LCNT::Register>),
        (0x24 => pub ic_hs_scl_hcnt: ReadWrite<u32, IC_HS_SCL_HCNT::Register>),
        (0x28 => pub ic_hs_scl_lcnt: ReadWrite<u32, IC_HS_SCL_LCNT::Register>),
        (0x2C => pub ic_intr_stat: ReadOnly<u32, IC_INTR_STAT::Register>),
        (0x30 => pub ic_intr_mask: ReadWrite<u32, IC_INTR_MASK::Register>),
        (0x34 => pub ic_raw_intr_stat: ReadOnly<u32, IC_RAW_INTR_STAT::Register>),
        (0x38 => pub ic_rx_tl: ReadWrite<u32, IC_RX_TL::Register>),
        (0x3C => pub ic_tx_tl: ReadWrite<u32, IC_TX_TL::Register>),
        (0x40 => pub ic_clr_intr: ReadOnly<u32, IC_CLR_INTR::Register>),
        (0x44 => pub ic_clr_rx_under: ReadOnly<u32, IC_CLR_RX_UNDER::Register>),
        (0x48 => pub ic_clr_rx_over: ReadOnly<u32, IC_CLR_RX_OVER::Register>),
        (0x4C => pub ic_clr_tx_over: ReadOnly<u32, IC_CLR_TX_OVER::Register>),
        (0x50 => pub ic_clr_rd_req: ReadOnly<u32, IC_CLR_RD_REQ::Register>),
        (0x54 => pub ic_clr_tx_abrt: ReadOnly<u32, IC_CLR_TX_ABRT::Register>),
        (0x58 => pub ic_clr_rx_done: ReadOnly<u32, IC_CLR_RX_DONE::Register>),
        (0x5C => pub ic_clr_activity: ReadOnly<u32, IC_CLR_ACTIVITY::Register>),
        (0x60 => pub ic_clr_stop_det: ReadOnly<u32, IC_CLR_STOP_DET::Register>),
        (0x64 => pub ic_clr_start_det: ReadOnly<u32, IC_CLR_START_DET::Register>),
        (0x68 => pub ic_clr_gen_call: ReadOnly<u32, IC_CLR_GEN_CALL::Register>),
        (0x6C => pub ic_enable: ReadOnly<u32, IC_ENABLE::Register>),
        (0x70 => pub ic_status: ReadOnly<u32, IC_STATUS::Register>),
        (0x74 => pub ic_txflr: ReadOnly<u32, IC_TXFLR::Register>),
        (0x78 => pub ic_rxflr: ReadOnly<u32, IC_RXFLR::Register>),
        (0x7C => pub ic_sda_hold: ReadWrite<u32, IC_SDA_HOLD::Register>),
        (0x80 => pub ic_tx_abrt_source: ReadOnly<u32, IC_TX_ABRT_SOURCE::Register>),
        (0x84 => pub ic_slv_data_nack_only: ReadWrite<u32, IC_SLV_DATA_NACK_ONLY::Register>),
        (0x88 => pub ic_dma_cr: ReadWrite<u32, IC_DMA_CR::Register>),
        (0x8C => pub ic_dma_tdlr: ReadWrite<u32, IC_DMA_TDLR::Register>),
        (0x90 => pub ic_dma_rdlr: ReadWrite<u32, IC_DMA_RDLR::Register>),
        (0x94 => pub ic_sda_setup: ReadWrite<u32, IC_SDA_SETUP::Register>),
        (0x98 => pub ic_ack_general_call: ReadWrite<u32, IC_ACK_GENERAL_CALL::Register>),
        (0x9C => pub ic_enable_status: ReadOnly<u32, IC_ENABLE_STATUS::Register>),
        (0xA0 => pub ic_fs_spklen: ReadWrite<u32, IC_FS_SPKLEN::Register>),
        (0xA4 => pub ic_hs_spklen: ReadWrite<u32, IC_HS_SPKLEN::Register>),
        (0xA8 => @END)
    }
}

pub fn i2c_get_enable () -> bool {
    let enable = I2C_REGS.ic_enable_status.is_set(IC_ENABLE_STATUS::IC_EN::CONTROLLER_OPENED);
    info!("I2C: get enable {}", enable);
    enable
}

pub fn i2c_set_enable (enable: bool) {
    let mut timeout = I2C_TIMEOUT;
    while (timeout > 0) {
        if enable {
            I2C_REGS.ic_enable.set(IC_ENABLE::ENABLE);
            let success = I2C_REGS.ic_enable_status.is_set(IC_ENABLE_STATUS::IC_EN::CONTROLLER_OPENED);
            if success {
                info!("I2C: set enable success");
                return;
            }
        } else {
            I2C_REGS.ic_enable.set(IC_ENABLE::DISABLE);
            let success = I2C_REGS.ic_enable_status.is_set(IC_ENABLE_STATUS::IC_EN::CONTROLLER_CLOSED);
            if success {
                info!("I2C: set disable success");
                return;
            }
        }
        timeout -= 1;
    }
    error!("I2C: set enable {} timeout", enable);
}

pub fn i2c_intr_stat() -> u32 {
    I2C_REGS.ic_intr_stat.read()
}

pub fn i2c_clear_intr_stat () {
    I2C_REGS.ic_clr_intr.read();
}

/// Clear the interrupt status bit,
/// return the interrupt status before clearing
pub fn i2c_clear_intr_bits (last_err: &mut u32) -> u32 {
    let stat = i2c_intr_stat();
    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_TX_ABRT)) {
        *last_err = I2C_REGS.ic_tx_abrt_source.read();
        I2C_REGS.ic_clr_tx_abrt.read(IC_CLR_TX_ABRT::CLR_TX_ABRT);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_RX_UNDER)) {
        I2C_REGS.ic_clr_rx_under.read(IC_CLR_RX_UNDER::CLR_RX_UNDER);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_RX_OVER)) {
        I2C_REGS.ic_clr_rx_over.read(IC_CLR_RX_OVER::CLR_RX_OVER);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_TX_OVER)) {
        I2C_REGS.ic_clr_tx_over.read(IC_CLR_TX_OVER::CLR_TX_OVER);
    }
    
    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_RX_DONE)) {
        I2C_REGS.ic_clr_rx_done.read(IC_CLR_RX_DONE::CLR_RX_DONE);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_ACTIVITY)) {
        I2C_REGS.ic_clr_activity.read(IC_CLR_ACTIVITY::CLR_ACTIVITY);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_STOP_DET)) {
        I2C_REGS.ic_clr_stop_det.read(IC_CLR_STOP_DET::CLR_STOP_DET);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_START_DET)) {
        I2C_REGS.ic_clr_start_det.read(IC_CLR_START_DET::CLR_START_DET);
    }

    if (I2C_REGS.ic_intr_stat.is_set(IC_INTR_STAT::R_GEN_CALL)) {
        I2C_REGS.ic_clr_gen_call.read(IC_CLR_GEN_CALL::CLR_GEN_CALL);
    }

    stat
}

/// Clear the abnormal status bit
pub fn i2c_clear_abort () {
    let mut timeout = I2C_TIMEOUT;
    while (timeout > 0) {
        i2c_clear_intr_stat();
        if (I2C_REGS.ic_tx_abrt_source.read() == 0) {
            return;
        }
        timeout -= 1;
    }
    error!("I2C: clear abort failed");
}

/// Wait for a specific I2C status bit until 
/// the status does not exist or a timeout occurs.
pub fn i2c_wait_status (status_bits: u32) {
    let mut timeout: u32 = 0;
    while (!(I2C_REGS.ic_status.read(IC_STATUS) & status_bits) && (timeout < I2C_TIMEOUT)) {
        timeout += 1;
    }

    if (timeout >= I2C_TIMEOUT) {
        error!("I2C: wait status timeout, status: {:x}", status_bits);
    }
}

/// Wait for I2C bus busy
pub fn i2c_wait_bus_busy () {
    i2c_wait_status(IC_STATUS::TFE);
    if (I2C_REGS.ic_status.is_set(IC_STATUS::MST_ACTIVITY)) {
        error!("I2C: Timeout when wait i2c bus busy");
    }
}

pub fn i2c_read_data () -> u8 {
    (I2C_DATA_MASK & (I2C_REGS.ic_data_cmd.read(IC_DATA_CMD::DAT) as u8) ) as u8
}

/// Waiting for the completion of Fifo transmission
pub fn i2c_flush_rx_fifo () {
    let mut timeout = 0;

    /* read data to trigger trans until fifo empty */
    while (I2C_REGS.ic_status.is_set(IC_STATUS::RFNE)) {
        i2c_read_data();
        if (timeout >= I2C_TIMEOUT) {
            error!("I2C: flush rx fifo timeout");
        }
        timeout += 1;
    }
}

pub fn i2c_ctrl_disable () {
    I2C_REGS.ic_rx_tl.write(0);
    I2C_REGS.ic_tx_tl.write(0);
    /* disable all intr */
    I2C_REGS.ic_intr_mask.write(0);
    i2c_set_enable(false);
    info!("I2C: set ctrl disable successfully");
}

