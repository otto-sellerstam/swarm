use defmt_rtt as _;
use embassy_futures::select::{Either, select};
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{
    Common, Config, Direction as PioDir, FifoJoin, Instance, PioPin, ShiftConfig, ShiftDirection,
    StateMachine,
};
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;

pub struct RotaryEncoder<'d, P: Instance, const SM: usize> {
    sm: StateMachine<'d, P, SM>,
    sw_input: Input<'d>,
    state: State,
}

pub enum Event {
    RotationClockwise,
    RotationAntiClockwise,
    PressDown,
    PressUp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum State {
    Start,
    CWBegin,
    CWNext,
    CWFinal,
    CCWBegin,
    CCWNext,
    CCWFinal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    None,
    CW,
    CCW,
}

const BEN_BUXTON_TABLE: [[(State, Direction); 4]; 7] = [
    [
        (State::Start, Direction::None),
        (State::CWBegin, Direction::None),
        (State::CCWBegin, Direction::None),
        (State::Start, Direction::None),
    ],
    [
        (State::CWNext, Direction::None),
        (State::CWBegin, Direction::None),
        (State::Start, Direction::None),
        (State::Start, Direction::None),
    ],
    [
        (State::CWNext, Direction::None),
        (State::CWBegin, Direction::None),
        (State::CWFinal, Direction::None),
        (State::Start, Direction::None),
    ],
    [
        (State::CWNext, Direction::None),
        (State::Start, Direction::None),
        (State::CWFinal, Direction::None),
        (State::Start, Direction::CW),
    ],
    [
        (State::CCWNext, Direction::None),
        (State::Start, Direction::None),
        (State::CCWBegin, Direction::None),
        (State::Start, Direction::None),
    ],
    [
        (State::CCWNext, Direction::None),
        (State::CCWFinal, Direction::None),
        (State::CCWBegin, Direction::None),
        (State::Start, Direction::None),
    ],
    [
        (State::CCWNext, Direction::None),
        (State::CCWFinal, Direction::None),
        (State::Start, Direction::None),
        (State::Start, Direction::CCW),
    ],
];

impl<'d, P: Instance, const SM: usize> RotaryEncoder<'d, P, SM> {
    pub fn new(
        common: &mut Common<'d, P>,
        mut sm: StateMachine<'d, P, SM>,
        clk_pin: Peri<'d, impl PioPin>,
        dt_pin: Peri<'d, impl PioPin>, // Needs to be clk_pin - 1
        sw_pin: Peri<'d, impl Pin>,
    ) -> Self {
        let prg = pio_asm!(
            "top:",
            "    mov isr, null",
            "    in pins, 2",
            "    mov x, isr",
            "    jmp x!=y, changed",
            "    jmp top",
            "changed:",
            "    mov y, x",
            "    push block",
            "    jmp top",
        );
        let loaded = common.load_program(&prg.program);

        let mut clk = common.make_pio_pin(clk_pin);
        let mut dt = common.make_pio_pin(dt_pin);
        clk.set_pull(Pull::Up);
        dt.set_pull(Pull::Up);

        let mut cfg = Config::default();
        cfg.use_program(&loaded, &[]);
        cfg.set_in_pins(&[&clk, &dt]);
        cfg.shift_in = ShiftConfig {
            auto_fill: false,
            threshold: 32,
            direction: ShiftDirection::Left,
        };
        cfg.fifo_join = FifoJoin::RxOnly;
        cfg.clock_divider = 1_000_u16.to_fixed();

        sm.set_config(&cfg);
        sm.set_pin_dirs(PioDir::In, &[&clk, &dt]);
        sm.set_enable(true);

        let sw_input = Input::new(sw_pin, Pull::Up);

        Self {
            sm,
            sw_input,
            state: State::Start,
        }
    }

    fn buxton_lookup(state: State, reading: usize) -> (State, Direction) {
        BEN_BUXTON_TABLE[state as usize][reading]
    }

    pub async fn next_event(&mut self) -> Event {
        loop {
            match select(self.sw_input.wait_for_any_edge(), self.sm.rx().wait_pull()).await {
                Either::First(_) => {
                    Timer::after(Duration::from_millis(20)).await;
                    if self.sw_input.is_low() {
                        return Event::PressDown;
                    } else {
                        return Event::PressUp;
                    }
                }
                Either::Second(reading) => {
                    let (next_rotary_state, direction) =
                        Self::buxton_lookup(self.state, reading as usize);
                    self.state = next_rotary_state;
                    match direction {
                        Direction::None => continue,
                        Direction::CW => return Event::RotationClockwise,
                        Direction::CCW => return Event::RotationAntiClockwise,
                    }
                }
            };
        }
    }
}
