use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

const ADDR: u8 = 0x76;
const REG_BLOCK1: u8 = 0x88;
const REG_BLOCK2: u8 = 0xE1;
const REG_CTRL_HUM: u8 = 0xF2;
const REG_CONFIG: u8 = 0xF5;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_PRESS_MSB: u8 = 0xF7; // The first measurement of 8; each one byte.

struct Measurement {
    temp: f32,
    humidity: f32,
    pressure: f32,
}

#[derive(Default)]
struct Calibration {
    // Block 1.
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    dig_h1: u8,

    // Block 2.
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16, // 12 bit-packed.
    dig_h5: i16, // 12 bit-packed.
    dig_h6: i8,
}

impl Calibration {
    fn from_raw(block1: [u8; 26], block2: [u8; 7]) -> Self {
        let u = |i: usize| u16::from_le_bytes([block1[i], block1[i + 1]]);
        let s = |i: usize| i16::from_le_bytes([block1[i], block1[i + 1]]);

        let e4 = block2[3] as u16;
        let e5 = block2[4] as u16;
        let e6 = block2[5] as u16;

        let h4_raw = (e4 << 4) | (e5 & 0x0F);
        let dig_h4 = ((h4_raw << 4) as i16) >> 4;

        let h5_raw = (e6 << 4) | (e5 >> 4);
        let dig_h5 = ((h5_raw << 4) as i16) >> 4;

        Self {
            dig_t1: u(0),
            dig_t2: s(2),
            dig_t3: s(4),
            dig_p1: u(6),
            dig_p2: s(8),
            dig_p3: s(10),
            dig_p4: s(12),
            dig_p5: s(14),
            dig_p6: s(16),
            dig_p7: s(18),
            dig_p8: s(20),
            dig_p9: s(22),
            dig_h1: block1[25],
            dig_h2: i16::from_le_bytes(block2[0..2].try_into().unwrap()),
            dig_h3: block2[2],
            dig_h4,
            dig_h5,
            dig_h6: block2[6] as i8,
        }
    }
}

struct Bme280<I2C> {
    i2c: I2C,
    cal: Calibration,
}

impl<I2C: I2c> Bme280<I2C> {
    fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            cal: Calibration::default(),
        }
    }

    async fn read_reg(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c.write_read(ADDR, &[reg], buf).await
    }

    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), I2C::Error> {
        self.i2c.write(ADDR, &[reg, val]).await
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        // First we read and set the calibration.
        let mut block1 = [0_u8; 26];
        self.read_reg(REG_BLOCK1, &mut block1).await?;

        let mut block2 = [0_u8; 7];
        self.read_reg(REG_BLOCK2, &mut block2).await?;

        self.cal = Calibration::from_raw(block1, block2);

        // Then we write the config.
        self.write_reg(REG_CTRL_HUM, 0x01).await?; // ctrl_hum: humidity x1 oversampling.
        self.write_reg(REG_CONFIG, 0x00).await // config: filter off.
    }

    async fn read(&mut self) -> Result<(), I2C::Error> {
        // Since we use force mode, we write ctrl_meas as the trigger.
        self.write_reg(REG_CTRL_MEAS, 0x25).await?; // 0x25 = 001-001-01.
        Timer::after(Duration::from_millis(10)).await; // We wait 10 ms before fetching.

        let mut buf = [0_u8; 8];
        self.read_reg(REG_PRESS_MSB, &mut buf).await
    }
}
