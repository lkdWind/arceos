use core::sync::atomic::{AtomicU32, Ordering, AtomicBool};
use alloc::Vec::Vec;
use i2c_regs::*;

const NANO_TO_KILO: u32 = 1000000;
const I2C_CON_SPEED_MASK: u32 = gen_mask(2,1);
const I2C_IC_SAR_MASK: u32 = gen_mask(9,0);
const I2C_IC_TAR_MASK: u32 = gen_mask(9,0);
const I2C_DATA_MASK: u32 = gen_mask(7,0);

const I2C_SPEED_STANDARD_RATE: u32 = 100000;    /* 100kb/s */
const I2C_SPEED_FAST_RATE: u32 = 400000;        /* 400kb/s */
const I2C_SPEED_HIGH_RATE: u32 = 1000000;       /* 1Mkb/s */

const I2C_IIC_FIFO_MAX_LVL: u32 = 8;

pub struct I2cConfig {
    instance_id: u32,
    base_addr: u32,
    irq_num: u32,
    irq_prority: u32,
    ref_clk_hz: u32,
    master_mode: bool,
    slave_addr: u32,
    use_7bit_addr: bool,
    speed_rate: u32,
    auto_calculate: bool,
}

impl I2cConfig {
    pub fn new() -> Self {
        Self {
            instance_id: 0,
            base_addr: 0,
            irq_num: 0,
            irq_prority: 0,
            ref_clk_hz: 0,
            master_mode: true,
            slave_addr: 0,
            use_7bit_addr: true,
            speed_rate: 0,
            auto_calculate: true,
        }
    }
}

/// I2C Transmit Frame Structure
#[repr(C)]
#[derive(Debug)]
pub struct I2cTxFrame {
    /// Pointer to transmit data buffer (read-only)
    pub data_buff: Vec<u8>,
    /// Counter for transmitted bytes (atomic for thread safety)
    /// 
    /// This gets incremented as bytes are successfully sent.
    /// Atomic operations ensure safe access from interrupts/DMA.
    pub tx_cnt: AtomicU32,
    
    /// Control flags for frame configuration:
    /// - Bit 8: Command flag (special protocol handling)
    /// - Bit 9: Generate STOP condition after transfer
    /// - Bit 10: Generate RESTART condition before transfer
    pub flags: u32,
}

impl I2cTxFrame {
    pub fn new() -> Self {
        Self {
            data_buff: Vec::new(),
            tx_cnt: AtomicU32::new(0),
            flags: 0,
        }
    }

    pub fn push_byte(&mut self, byte: u8) {
        self.data_buff.push(byte);
    }
}

/// I2C Receive Frame Structure
#[repr(C)]
#[derive(Debug)]
pub struct I2cRxFrame {
    /// Pointer to receive data buffer (write-only)
    pub data_buff: Vec<u8>,
    pub current_pos: u32,
    
    /// Counter for received bytes (atomic for thread safety)
    /// 
    /// This gets incremented as bytes are successfully received.
    /// Atomic operations ensure safe access from interrupts/DMA.
    pub rx_cnt: AtomicU32,
}

impl I2cRxFrame {
    pub fn new() -> Self {
        Self {
            data_buff: Vec::new(),
            current_pos: 0,
            rx_cnt: AtomicU32::new(0),
        }
    }

    pub fn next_byte(&self) -> Option<u8> {
        if self.current_pos < self.data_buff.len() {
            let byte = self.data_buff[self.current_pos];
            self.current_pos += 1;
            Some(byte)
        } else {
            None
        }
    }
}

enum I2cMode{
    Master,
    Slave,
}

enum I2cStatus {
    Idle,
    WriteInProgess,
    ReadInProgess,
    Error,
}

enum I2cMasterEvt {
    MasterTransAborted,
    MasterReadDone,
    MasterWriteDone,
}

pub trait I2cMasterEvtHandler {
    pub fn on_trans_aborted() {
        info!("I2C: Master Trans aborted handler triggered");
    }
    pub fn on_read_done() {
        info!("I2C: Master Read done handler triggered");
    }
    pub fn on_write_done() {
        info!("I2C: Master Write done handler triggered");
    }
}

enum I2cSlaveEvt {
    SlaveReadRequested,
    SlaveWriteRequested,
    SlaveReadProcessed,
    SlaveWriteProcessed,
    SlaveStop,
    SlaveAbort,
}

pub trait I2cSlaveEvtHandler{
    pub fn on_read_requested() {
        info!("I2C: Slave Read requested handler triggered");
    }
    pub fn on_write_requested() {
        info!("I2C: Slave Write requested handler triggered");
    }
    pub fn on_read_processed() {
        info!("I2C: Slave Read processed handler triggered");
    }
    pub fn on_write_processed() {
        info!("I2C: Slave Write processed handler triggered");
    }
    pub fn on_stop() {
        info!("I2C: Slave Stop handler triggered");
    }
    pub fn on_abort() {
        info!("I2C: Slave Abort handler triggered");
    }
}

enum I2cSpeedMode {
    Standard,
    Quick,
    High,
}

/// speed configs
pub struct I2cSpeedCfg {
    pub speed_mode: I2cSpeedMode,
    pub scl_lcnt: u32,
    pub scl_hcnt: u32,
    pub sda_hold: u32,
}

pub struct I2cSpeedModeInfo {
    pub min_scl_high_time_ns: u32,
    pub min_scl_low_time_ns: u32,
    pub def_rise_time_ns: u32,
    pub def_fall_time_ns: u32,
}

impl I2cSpeedModeInfo {
    pub fn get_speed_mode_info(speed_mode: I2cSpeedMode) -> Self {
        match speed_mode {
            I2cSpeedMode::Standard => Self {
                min_scl_high_time_ns: 4000,
                min_scl_low_time_ns: 4700,
                def_rise_time_ns: 1000,
                def_fall_time_ns: 300,
            },
            I2cSpeedMode::Quick => Self {
                min_scl_high_time_ns: 600,
                min_scl_low_time_ns: 1300,
                def_rise_time_ns: 300,
                def_fall_time_ns: 300,                    
            },
            I2cSpeedMode::High => Self {
                min_scl_high_time_ns: 390,
                min_scl_low_time_ns: 460,
                def_rise_time_ns: 60,
                def_fall_time_ns: 160,
            },
        }
    }
}

/// I2C rate configuration register value, 
/// specific configuration parameters can be adjusted
/// SCL_FREQ = I2C_CLK_FREQ / (SCL_L_CNT + SCL_H_CNT + 1 + 7 +I2C_SPKLEN)
impl I2cSpeedCfg {
    pub fn new() -> Self {
        Self {
            speed_mode: None,
            scl_lcnt: 0,
            scl_hcnt: 0,
            sda_hold: 0,
        }
    }

    pub fn set_default_cfg(&mut self) {
        match self.speed_mode {
            I2cSpeedMode::Standard => {
                self.scl_lcnt = 232;
                self.scl_hcnt = 205;
                self.sda_hold = 15; 
            },
            I2cSpeedMode::Quick => {
                self.scl_lcnt = 79;
                self.scl_hcnt = 18;
                self.sda_hold = 15;            
            },
            I2cSpeedMode::High => {
                self.scl_lcnt = 31;
                self.scl_hcnt = 6;
                self.sda_hold = 15;                 
            }
            None => {
                error!("I2C speed mode is not set");
            }
        }
    }
}

pub struct I2c {
    pub config: I2cConfig,
    pub is_ready: AtomicBool,
    pub status: I2cStatus,
    pub tx_frame: I2cTxFrame,
    pub rx_frame: I2cRxFrame,
}

impl I2c {
    pub fn new () -> Self {
        Self {
            i2c_cfg: I2cConfig::new(),
            is_ready: AtomicBool::new(false),
            status: I2cStatus::Idle,
            txframe: I2cTxFrame::new(),
            rxframe: I2cRxFrame::new(),
        }
    }

    pub fn cfg_init(&mut self) {
        if (self.is_ready.load(Ordering::Relaxed)) {
            warn!("I2C is already initialized");
            return
        }
        self = I2c::new();
        self.is_ready.store(true, Ordering::Relaxed);
        info!("I2C initialized successfully");
    }

    pub fn calc_timing (&mut self, spk_num: u32, speed_cfg: &mut I2cSpeedCfg) -> u32 {
        let speed_info = I2cSpeedModeInfo::get_speed_mode_info(speed_cfg.speed_mode);
        let bus_clk_hz = self.config.bus_clk_hz;
        let period_cnt = bus_clk_hz / self.config.speed_rate;

        // convert a period to a number of IC clk cycles
        let rise_cnt = div_round_up(bus_clk_hz / 1000 * speed_info.def_risetime_ns, NANO_TO_KILO);
        let fall_cnt = div_round_up(bus_clk_hz / 1000 * speed_info.def_falltime_ns, NANO_TO_KILO);
        let min_t_low_cnt = div_round_up(bus_clk_hz / 1000 * speed_info.min_scl_low_time_ns, NANO_TO_KILO);
        let min_t_high_cnt = div_round_up(bus_clk_hz / 1000 * speed_info.min_scl_high_time_ns, NANO_TO_KILO);
        info!("I2c: mode {:?}, bus_clk_hz {}, speed {}, period {}", speed_cfg.speed_mode, bus_clk_hz, self.config.speed_rate, period_cnt);
        info!("I2c: rise {}, fall {}, min_t_low {}, min_t_high {}, spk_num {}", rise_cnt, fall_cnt, min_t_low_cnt, min_t_high_cnt, spk_num);
        /*
         * Back-solve for hcnt and lcnt according to the following equations:
         * SCL_High_time = [(HCNT + IC_*_SPKLEN + 7) * icClk] + SCL_Fall_time
         * SCL_Low_time = [(LCNT + 1) * icClk] - SCL_Fall_time + SCL_Rise_time
         */
        let mut hcnt = min_t_high_cnt - fall_cnt - 7 - spk_num;
        let mut lcnt = min_t_low_cnt - rise_cnt - fall_cnt - 1;

        if (hcnt < 0 || lcnt < 0) {
            error!("I2c: bad counts. hcnt {}, lcnt {}", hcnt, lcnt);
            return;
        }
        /*
         * Now add things back up to ensure the period is hit. If it is off,
         * split the difference and bias to lcnt for remainder
         */
        let mut tot = hcnt + lcnt + 7 + spk_num + rise_cnt + 1;
        let diff: u32 = 0;
        if (tot < period_cnt) {
            diff = (period_cnt - tot) / 2;
            hcnt += diff;
            lcnt += diff;
            tot = hcnt + lcnt + 7 + spk_num + rise_cnt + 1;
            lcnt += period_cnt - tot;
        }

        speed_cfg.scl_hcnt = hcnt;
        speed_cfg.scl_lcnt = lcnt;
        /* Use internal default unless other value is specified */
        speed_cfg.sda_hold = div_round_up(bus_clk_hz / 1000 * 300, NANO_TO_KILO);

        info!("I2c: hcnt {}, lcnt {}, sda_hold {}", speed_cfg.scl_hcnt, speed_cfg.scl_lcnt, speed_cfg.sda_hold);
        info!("I2c calcalated timing successfully");
    }

    pub fn set_timing (&self, &speed_cfg: I2cSpeedCfg) {
        let mut reg_con_val = I2C_REGS.ic_con.read();
        reg_con_val = (reg_con_val & !I2C_CON_SPEED_MASK);
        match speed_cfg.speed_mode {
            I2cSpeedMode::Standard => {
                I2C_REGS.ic_con.modify(IC_CON::SPEED::STANDARD);
                I2C_REGS.ic_ss_scl_hcnt.write(speed_cfg.scl_hcnt);
                I2C_REGS.ic_ss_scl_lcnt.write(speed_cfg.scl_lcnt);
            }
            I2cSpeedMode::Quick => {
                I2C_REGS.ic_con.modify(IC_CON::SPEED::QUICK);
                I2C_REGS.ic_fs_scl_hcnt.write(speed_cfg.scl_hcnt);
                I2C_REGS.ic_fs_scl_lcnt.write(speed_cfg.scl_lcnt);
            }
            I2cSpeedMode::HIGH => {
                I2C_REGS.ic_con.modify(IC_CON::SPEED::HIGH);
                I2C_REGS.ic_hs_scl_hcnt.write(speed_cfg.scl_hcnt);
                I2C_REGS.ic_hs_scl_lcnt.write(speed_cfg.scl_lcnt);
            }
        }
        I2C_REGS.ic_sda_hold.write(speed_cfg.sda_hold);
        info!("I2c set timing successfully");
    }

    pub fn set_speed (&self, speed_rate: u32, auto_calc: bool) {
        let mut speed_cfg = I2cSpeedCfg::new();
        self.config.auto_calc = auto_calc;
        let spk_num: u32 = 0;
        if (I2C_SPEED_HIGH_RATE <= speed_rate) {
            speed_cfg.speed_mode = I2cSpeedMode::High;
            self.config.speed_rate = speed_rate;
            spk_num = I2C_REGS.ic_hs_spklen.read();
        } else if (I2C_SPEED_QUICK_RATE <= speed_rate) {
            speed_cfg.speed_mode = I2cSpeedMode::Quick;
            self.config.speed_rate = speed_rate;
            spk_num = I2C_REGS.ic_fs_spklen.read();
        } else if (I2C_SPEED_STANDARD_RATE <= speed_rate) {
            speed_cfg.speed_mode = I2cSpeedMode::Standard;
            self.config.speed_rate = speed_rate;
            spk_num = I2C_REGS.ic_ss_spklen.read();
        } else {
            error!("I2c: bad speed rate {}", speed_rate);
            return;
        }

        /*  disable setting for restore later */
        let is_enable = I2C_REGS.ic_enable.is_set(IC_ENABLE::ENABLE);
        if (is_enable) {
            I2C_REGS.ic_enable.write(IC_ENABLE::DISABLE);
        }
        if (auto_calc) {
            self.calc_timing(spk_num, &mut speed_cfg);
        } else {
            speed_cfg.set_default_cfg();
        }

        self.set_timing(speed_cfg);
        if (is_enable) {
            I2C_REGS.ic_enable.write(IC_ENABLE::ENABLE);
        }
        info!("I2c set speed successfully");
    }

    pub fn set_tar (&self, tar_addr: u32) {
        let is_enable = i2c_get_enable();
        if (is_enable) {
            i2c_set_enable(false);
        }
        I2C_REGS.ic_tar.write(IC_TAR::IC_TAR.val(tar_addr & I2C_IC_TAR_MASK));
        info!(" I2c set tar addr {:x}", tar_addr);

        if !self.config.use_7bit_addr {
            I2C_REGS.ic_tar.modify(IC_TAR::IC_10BITADDR_MASTER::TEN_BITS_ADDR_MODE);
            info!(" I2c set 10bit addr successfully");
        } else {
            I2C_REGS.ic_tar.modify(IC_TAR::IC_10BITADDR_MASTER::SEVEN_BITS_ADDR_MODE);  
            info!(" I2c set 7bit addr successfully");         
        }

        if (is_enable) {
            i2c_set_enable(true);
        }
        info!("I2c set tar successfully");
    }

    pub fn set_sar(sar_addr: u32) {
        let is_enable = i2c_get_enable();
        if (is_enable) {
            i2c_set_enable(false);
        }
        I2C_REGS.ic_sar.write(IC_SAR::IC_SAR.val(sar_addr & I2C_IC_SAR_MASK));
        info!(" I2c set sar addr {:x}", sar_addr);
        if (is_enable) {
            i2c_set_enable(true);
        }
        info!("I2c set sar successfully");
    }

    pub fn set_addr(&mut self, work_mode: I2cWorkMode, slave_addr: u32) {
        let is_enable = i2c_get_enable();
        if (is_enable) {
            i2c_set_enable(false);
        }
        match work_mode {
            I2cWorkMode::Master => {
                /* FI2C_CON_MASTER_ADR_7BIT : FI2C_CON_MASTER_ADR_10BIT 主机配置7位或10位地址方式，需要在 FI2C_TAR_OFFSET 第12位设置，FI2cSetTar()函数会设置 */
                I2C_REGS.ic_con.modify(IC_CON::IC_SLAVE_DISABLE::ENABLE
                                    + IC_CON::MASTER_MODE::ENABLE
                                    + IC_CON::IC_RESTART_EN::ENABLE);
            }
            I2cWorkMode::Slave => {
                if (self.config.use_7bit_addr) { 
                    I2C_REGS.ic_con.modify(IC_CON::IC_10BITADDR_SLAVE::SEVEN_BITS_ADDR_MODE);
                } else {
                    I2C_REGS.ic_con.modify(IC_CON::IC_10BITADDR_SLAVE::TEN_BITS_ADDR_MODE);
                }
                I2C_REGS.ic_con.modify(IC_CON::MASTER_MODE::DISABLE
                                    + IC_CON::IC_SLAVE_DISABLE::DISABLE);
            }
        }
        self.config.slave_addr = slave_addr;
        self.config.work_mode = work_mode;

        I2C_REGS.ic_rx_tl.write(IC_RX_TL::RX_TL.val(0));
        I2C_REGS.ic_tx_tl.write(IC_TX_TL::TX_TL.val(0));
        /* disable all intr */
        I2C_REGS.ic_intr_mask.write(0);
        
        i2c_set_enable(true);
        /* if init successed, and i2c is in slave mode, set slave address */
        if (work_mode == I2cWorkMode::Slave) {
            self.set_tar(self.config.slave_addr);
        }
        info!("I2c set addr successfully, mode: {:?}, addr: {:x}", work_mode, slave_addr);
    }

    /// Default callback function under the master mode of I2C interrupt
    pub fn stub_handler() {
        info!("I2c stub handler, intr cause {:x}", I2C_REGS.ic_intr_stat.read());
    }

    /// I2C TX_FIFO interrupt handling function under master mode
    pub fn master_intr_tx_empty_handler(&mut self) {
        let mut buf_len = self.txframe.data_buffer.len() - self.txframe.tx_cnt;
        let mut rx_limit = I2C_IIC_FIFO_MAX_LVL - I2C_REGS.ic_rxflr.read();
        let mut tx_limit = I2C_IIC_FIFO_MAX_LVL - I2C_REGS.ic_txflr.read();

        while (buf_len > 0 && tx_limit > 0 && rx_limit > 0) {
            if (1 == buf_len) {
                match self.status {
                    I2cStatus::WriteInProgess => {
                        let data = instance.txframe.next_byte()
                                .map(|b| (b & I2C_DATA_MASK) as u32)
                                .unwrap_or(0);
                        I2C_REGS.ic_data_cmd.modify(IC_DATA_CMD::DAT.val(data)
                                + IC_DATA_CMD::CMD::MASTER_WRITE_MODE
                                + IC_DATA_CMD::STOP::ENABLE);
                        info!("I2C: Write Stop Singal");
                    }
                    I2cStatus::ReadInProgess => {
                        I2C_REGS.ic_data_cmd.modify(IC_DATA_CMD::CMD::MASTER_READ_MODE 
                                + IC_DATA_CMD::STOP::ENABLE);
                    }
                }
            } else {
                match self.status {
                    I2cStatus::WriteInProgess => {
                        let data = instance.txframe.next_byte()
                                .map(|b| (b & I2C_DATA_MASK) as u32)
                                .unwrap_or(0);
                        I2C_REGS.ic_data_cmd.modify(IC_DATA_CMD::DAT.val(data)
                                + IC_DATA_CMD::CMD::MASTER_WRITE_MODE);
                    }
                    I2cStatus::ReadInProgess => {
                        I2C_REGS.ic_data_cmd.modify(IC_DATA_CMD::CMD::MASTER_READ_MODE);
                    }
                }
            }
            I2C_REGS.ic_data_cmd.modify(IC_DATA_CMD::DATA_CMD::IC_DATA_CMD);
            rx_limit =- 1;
            tx_limit =- 1;
            buf_len =- 1;
        }

        self.tx_frame.tx_cnt = self.txframe.data_buffer.len() - buf_len;

        if (self.tx_frame.tx_cnt == self.tx_frame.data_buffer.len()) {
            self.tx_frame.tx_cnt = 0;
            if (self.status == I2cStatus::WriteInProgess) {
                self.status = I2cStatus::Idle;
            }
            I2C_REGS.ic_intr_mask.modify(IC_INTR_MASK::M_TX_EMPTY::CLEAR);
        }
    }

    /// I2C RX_FIFO interrupt handling function under master mode
    pub fn master_intr_rx_full_handler(&mut self) {
        let empty_fifo = I2C_REGS.ic_rxflr.read();
        for i in 0..empty_fifo {
            self.rx_frame.data_buffer.push( I2C_REGS.ic_data_cmd.read(IC_DATA_CMD::DAT) as u8);
        }

        self.rx_frame.rx_cnt += empty_fifo;

        if (self.rx_frame.rx_cnt >= self.rx_frame.data_buffer.len()) {
            self.rx_frame.rx_cnt = 0;
            self.status = I2cStatus::Idle;
            I2C_REGS.ic_intr_mask.modify(IC_INTR_MASK::M_RX_FULL::CLEAR);   
            i2c_flush_rx_fifo();        
        }
    }

    pub fn master_intr_handler(&mut self) {
        if (self.work_mode != I2CWorkMode::Master) {
            error!("I2c master intr handler, but not in master mode");
            return
        }
        let mut last_err = 0;
        let stat = i2c_clear_intr_bits(&last_err);
        let raw_stat = I2C_REGS.ic_raw_intr_stat.read();
        let enabled = I2C_REGS.ic_enable.is_set(IC_ENABLE::ENABLE::ON_CONTROLLER);
        
        if (!enabled || !I2C_REGS.ic_raw_intr_stat.is_set(IC_RAW_INTR_STAT::ACTIVITY)) {
            return
        }

        /* trans abort error */
        if (stat & I2C_INTR_STAT::R_TX_ABRT) {
            error!("I2C last error {}", last_err);
            error!("I2C: abort source: {},please see the FI2C_TX_ABRT_SOURCE_OFFSET register",
                    I2C_REGS.ic_tx_abrt_source.read());
            self.status = I2cStatus::Error;

            /* disable all intr */
            I2C_REGS.ic_intr_mask.write(0);
            I2C_REGS.ic_clr_tx_abrt.read();
            I2C_REGS.ic_enable.modify(IC_ENABLE::ENABLE::ON_CONTROLLER);

            return
        }

        /* rx complete */
        if (stat & I2C_INTR_STAT::R_TX_FULL) {
            self.master_intr_rx_full_handler();

            return
        }

        /* tx complete */
        if (stat & I2C_INTR_STAT::R_TX_EMPTY) {
            self.master_intr_tx_empty_handler();

            return
        }
    }
}

impl I2cMasterEvtHandler for I2cMaster {}

impl I2cSlaveEvtHandler for I2cSlave {}