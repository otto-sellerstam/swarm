use defmt_rtt as _;
use embassy_futures::select::{Either3, select3};
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_time::{Duration, Timer};

pub struct RotaryEncoder {
    clk_input: Input<'static>,
    dt_input: Input<'static>,
    sw_input: Input<'static>,
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

impl RotaryEncoder {
    pub fn new<T: Pin, U: Pin, V: Pin>(
        clk_pin: Peri<'static, T>,
        dt_pin: Peri<'static, U>,
        sw_pin: Peri<'static, V>,
    ) -> Self {
        let clk_input = Input::new(clk_pin, Pull::Up);
        let dt_input = Input::new(dt_pin, Pull::Up);
        let sw_input = Input::new(sw_pin, Pull::Up);

        Self {
            clk_input,
            dt_input,
            sw_input,
            state: State::Start,
        }
    }

    fn rotary_reading(&self) -> usize {
        ((self.dt_input.is_high() as usize) << 1) | (self.clk_input.is_high() as usize)
    }

    pub async fn next_event(&mut self) -> Event {
        loop {
            match select3(
                self.sw_input.wait_for_any_edge(),
                self.clk_input.wait_for_any_edge(),
                self.dt_input.wait_for_any_edge(),
            )
            .await
            {
                Either3::First(_) => {
                    Timer::after(Duration::from_millis(20)).await;
                    if self.sw_input.is_low() {
                        return Event::PressDown;
                    } else {
                        return Event::PressUp;
                    }
                }
                Either3::Second(_) | Either3::Third(_) => {
                    let reading = self.rotary_reading();
                    let (next_rotary_state, direction) =
                        BEN_BUXTON_TABLE[self.state as usize][reading];
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
