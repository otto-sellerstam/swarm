use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

const ADDR: u8 = 0x76;
const REG_BLOCK1: u8 = 0x88;
const REG_BLOCK2: u8 = 0xE1;
const REG_CTRL_HUM: u8 = 0xF2;
const REG_CONFIG: u8 = 0xF5;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_PRESS_MSB: u8 = 0xF7; // The first measurement of 8; each one byte.
const REG_ID: u8 = 0xD0;
const BME280_ID: u8 = 0x60;

#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    pub temp: i32,     // Divide by 100 for Celsius
    pub humidity: u32, // Divide by 1024 for percent
    pub pressure: u32, // Divide by 256 for Pascals
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
    dig_h4: i16, // 12 bit-packed... nasty.
    dig_h5: i16, // 12 bit-packed... nasty.
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

    fn compensate_pressure(&self, adc_p: i32, t_fine: i32) -> u32 {
        let mut var1 = (t_fine as i64) - 128000;
        let mut var2 = var1 * var1 * (self.dig_p6 as i64);
        var2 += (var1 * (self.dig_p5 as i64)) << 17;
        var2 += (self.dig_p4 as i64) << 35;
        var1 = ((var1 * var1 * (self.dig_p3 as i64)) >> 8) + ((var1 * (self.dig_p2 as i64)) << 12);
        var1 = ((1i64 << 47) + var1) * (self.dig_p1 as i64) >> 33;

        if var1 == 0 {
            return 0; // avoid division by zero
        }

        let mut p: i64 = 1_048_576 - (adc_p as i64);
        p = (((p << 31) - var2) * 3125) / var1;
        var1 = ((self.dig_p9 as i64) * (p >> 13) * (p >> 13)) >> 25;
        var2 = ((self.dig_p8 as i64) * p) >> 19;
        p = ((p + var1 + var2) >> 8) + ((self.dig_p7 as i64) << 4);

        p as u32
    }

    fn compensate_humidity(&self, adc_h: i32, t_fine: i32) -> u32 {
        let mut v: i32 = t_fine - 76_800;

        v = (((adc_h << 14) - ((self.dig_h4 as i32) << 20) - ((self.dig_h5 as i32) * v) + 16_384)
            >> 15)
            * (((((((v * (self.dig_h6 as i32)) >> 10)
                * (((v * (self.dig_h3 as i32)) >> 11) + 32_768))
                >> 10)
                + 2_097_152)
                * (self.dig_h2 as i32)
                + 8_192)
                >> 14);

        v -= ((((v >> 15) * (v >> 15)) >> 7) * (self.dig_h1 as i32)) >> 4;

        v = v.clamp(0, 419_430_400);
        (v as u32) >> 12
    }

    fn compensate_temperature(&self, adc_t: i32) -> (i32, i32) {
        let var1 = ((adc_t >> 3) - ((self.dig_t1 as i32) << 1)) * (self.dig_t2 as i32) >> 11;
        let var2 = (((((adc_t >> 4) - (self.dig_t1 as i32))
            * ((adc_t >> 4) - (self.dig_t1 as i32)))
            >> 12)
            * (self.dig_t3 as i32))
            >> 14;
        let t_fine = var1 + var2;
        let temp = (t_fine * 5 + 128) >> 8;

        (temp, t_fine)
    }

    fn compensate(&self, buf: [u8; 8]) -> Measurement {
        let [
            press_msb,
            press_lsb,
            press_xlsb,
            temp_msb,
            temp_lsb,
            temp_xlsb,
            hum_msb,
            hum_lsb,
        ] = buf;

        let adc_t =
            ((temp_msb as i32) << 12) | ((temp_lsb as i32) << 4) | ((temp_xlsb as i32) >> 4);
        let adc_p =
            ((press_msb as i32) << 12) | ((press_lsb as i32) << 4) | ((press_xlsb as i32) >> 4);
        let adc_h = ((hum_msb as i32) << 8) | (hum_lsb as i32);

        let (temp, t_fine) = self.compensate_temperature(adc_t);
        let pressure = self.compensate_pressure(adc_p, t_fine);
        let humidity = self.compensate_humidity(adc_h, t_fine);

        Measurement {
            temp,
            humidity,
            pressure,
        }
    }
}

#[derive(Debug)]
pub enum Error<E> {
    Bus(E),
    WrongChip { found: u8 },
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Bus(e)
    }
}

pub struct Bme280<I2C> {
    i2c: I2C,
    cal: Calibration,
}

impl<I2C: I2c> Bme280<I2C> {
    async fn read_reg(i2c: &mut I2C, reg: u8, buf: &mut [u8]) -> Result<(), I2C::Error> {
        i2c.write_read(ADDR, &[reg], buf).await
    }

    async fn write_reg(i2c: &mut I2C, reg: u8, val: u8) -> Result<(), I2C::Error> {
        i2c.write(ADDR, &[reg, val]).await
    }

    pub async fn init(mut i2c: I2C) -> Result<Self, Error<I2C::Error>> {
        // Double check that id is as expected.
        let mut id_buf = [0_u8];
        Self::read_reg(&mut i2c, REG_ID, &mut id_buf).await?;

        match id_buf[0] {
            BME280_ID => {}
            id => return Err(Error::WrongChip { found: id }),
        }

        // First we read and set the calibration.
        let mut block1 = [0_u8; 26];
        Self::read_reg(&mut i2c, REG_BLOCK1, &mut block1).await?;

        let mut block2 = [0_u8; 7];
        Self::read_reg(&mut i2c, REG_BLOCK2, &mut block2).await?;

        // Then we write the config.
        Self::write_reg(&mut i2c, REG_CTRL_HUM, 0x01).await?; // ctrl_hum: humidity x1 oversampling.
        Self::write_reg(&mut i2c, REG_CONFIG, 0x00).await?; // config: filter off.

        Ok(Self {
            i2c,
            cal: Calibration::from_raw(block1, block2),
        })
    }

    pub async fn read(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        // Since we use force mode, we write ctrl_meas as the trigger.
        Self::write_reg(&mut self.i2c, REG_CTRL_MEAS, 0x25).await?; // 0x25 = OSRS_T_X1 << 5 | OSRS_P_X1 << 2 | MODE_FORCED, meaning x1 oversampling temp - x1 oversampling pres - forced mode.
        Timer::after(Duration::from_millis(10)).await; // We wait 10 ms before fetching.

        // Data is 8 bytes and starts at REG_PRESS_MSB.
        let mut buf = [0_u8; 8];
        Self::read_reg(&mut self.i2c, REG_PRESS_MSB, &mut buf).await?;

        Ok(self.cal.compensate(buf))
    }
}
