use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_hal_async::i2c::I2c;

const ADDR: u8 = 0x3C;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const FB_LEN: usize = WIDTH * (HEIGHT / 8);

// This is the init sequence for the SSD1306.
const INIT: [u8; 25] = [
    0xAE, // display off
    0xD5, 0x80, // clock divide ratio / oscillator frequency
    0xA8, 0x3F, // multiplex ratio = 63 (64 rows)
    0xD3, 0x00, // display offset = 0
    0x40, // start line = 0
    0x8D, 0x14, // charge pump enable
    0x20, 0x00, // memory addressing mode = horizontal
    0xA1, // segment remap (mirror X) — use 0xA0 if flipped
    0xC8, // COM scan direction remap (mirror Y) — use 0xC0 if flipped
    0xDA, 0x12, // COM pins config for 128x64
    0x81, 0xCF, // contrast
    0xD9, 0xF1, // pre-charge period
    0xDB, 0x40, // VCOMH deselect level
    0xA4, // resume display to RAM content
    0xA6, // normal (non-inverted)
    0xAF, // display on
];

pub struct Ssd1306<I2C> {
    i2c: I2C,
    buf: [u8; 1 + FB_LEN], // The first byte is a control byte.
}

impl<I2C: I2c> Ssd1306<I2C> {
    pub fn new(i2c: I2C) -> Self {
        let mut buf = [0_u8; 1 + FB_LEN];
        buf[0] = 0x40; // Control byte for pixel data, never overwritten.
        Self { i2c, buf }
    }

    async fn commands(&mut self, cmds: &[u8]) -> Result<(), I2C::Error> {
        debug_assert!(cmds.len() < 32);
        let mut packet = [0_u8; 32];
        packet[0] = 0x00; // Control byte for command stream.
        packet[1..=cmds.len()].copy_from_slice(cmds);
        self.i2c.write(ADDR, &packet[..=cmds.len()]).await
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.commands(&INIT).await
    }

    pub fn clear(&mut self) {
        self.buf[1..].fill(0); // Leave control byte intact.
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let idx = 1 + (y / 8) * WIDTH + x;
        let bit = 1 << (y % 8);
        if on {
            self.buf[idx] |= bit;
        } else {
            self.buf[idx] &= !bit;
        }
    }

    pub async fn flush(&mut self) -> Result<(), I2C::Error> {
        self.commands(&[0x21, 0x00, 0x7F, 0x22, 0x00, 0x07]).await?;
        self.i2c.write(ADDR, &self.buf).await
    }
}

impl<I2C: I2c> OriginDimensions for Ssd1306<I2C> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<I2C: I2c> DrawTarget for Ssd1306<I2C> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                self.set_pixel(coord.x as usize, coord.y as usize, color.is_on());
            }
        }
        Ok(())
    }
}
