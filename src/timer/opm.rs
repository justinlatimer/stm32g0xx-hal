//! # One-pulse Mode
use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};
use crate::timer::pins::TimerPin;
use crate::timer::*;
use core::marker::PhantomData;

pub trait OpmExt: Sized {
    fn opm(self, period: MicroSecond, rcc: &mut Rcc) -> Opm<Self>;
}

pub struct OpmPin<TIM, CH> {
    tim: PhantomData<TIM>,
    channel: PhantomData<CH>,
    clk: Hertz,
    delay: MicroSecond,
}

pub struct Opm<TIM> {
    tim: PhantomData<TIM>,
    clk: Hertz,
}

impl<TIM> Opm<TIM> {
    pub fn bind_pin<PIN>(&self, pin: PIN) -> OpmPin<TIM, PIN::Channel>
    where
        PIN: TimerPin<TIM>,
    {
        pin.setup();
        OpmPin {
            tim: PhantomData,
            channel: PhantomData,
            clk: self.clk,
            delay: 0.ms(),
        }
    }
}

macro_rules! opm {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident, $arr:ident $(,$arr_h:ident)*),)+) => {
        $(
            impl OpmExt for $TIMX {
                fn opm(self, period: MicroSecond, rcc: &mut Rcc) -> Opm<Self> {
                    $timX(self, period, rcc)
                }
            }

            fn $timX(tim: $TIMX, period: MicroSecond, rcc: &mut Rcc) -> Opm<$TIMX> {
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());

                let cycles_per_period = rcc.clocks.apb_tim_clk / period.into();
                let psc = (cycles_per_period - 1) / 0xffff;
                tim.psc.write(|w| unsafe { w.psc().bits(psc as u16) });

                let freq = (rcc.clocks.apb_tim_clk.0 / (psc + 1)).hz();
                let reload = period.cycles(freq);
                unsafe {
                    tim.arr.write(|w| w.$arr().bits(reload as u16));
                    $(
                        tim.arr.modify(|_, w| w.$arr_h().bits((reload >> 16) as u16));
                    )*
                }
                Opm {
                    clk: freq,
                    tim: PhantomData,
                }
            }

            impl Opm<$TIMX> {
                pub fn generate(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.cr1.write(|w| w.opm().set_bit().cen().set_bit());
                }
            }
        )+
    }
}

macro_rules! opm_hal {
    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxm:ident, $ocxfe:ident, $ccrx:ident),)+
    ) => {
        $(
            impl OpmPin<$TIMX, $CH> {
                pub fn enable(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                    self.setup();
                }

                pub fn disable(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.ccer.modify(|_, w| w.$ccxe().clear_bit());
                }

                pub fn set_delay(&mut self, delay: MicroSecond) {
                    self.delay = delay;
                    self.setup();
                }

                fn setup(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    let compare = if self.delay.0 > 0 {
                        self.delay.cycles(self.clk)
                    } else {
                        1
                    };
                    unsafe {
                        tim.$ccrx.write(|w| w.bits(compare));
                        tim.$ccmrx_output().modify(|_, w| w.$ocxm().bits(7).$ocxfe().set_bit());
                    }
                }
            }
        )+
    };
}

opm_hal! {
    TIM1: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM1: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM1: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM1: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
    TIM3: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM16: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM17: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
}

#[cfg(feature = "stm32g0x1")]
opm_hal! {
    TIM2: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM2: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM2: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM2: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
}

opm! {
    TIM1: (apbenr2, apbrstr2, tim1, tim1en, tim1rst, arr),
    TIM3: (apbenr1, apbrstr1, tim3, tim3en, tim3rst, arr_l, arr_h),
    TIM14: (apbenr2, apbrstr2, tim14, tim14en, tim14rst, arr),
    TIM16: (apbenr2, apbrstr2, tim16, tim16en, tim16rst, arr),
    TIM17: (apbenr2, apbrstr2, tim17, tim17en, tim17rst, arr),
}

#[cfg(feature = "stm32g0x1")]
opm! {
    TIM2: (apbenr1, apbrstr1, tim2, tim2en, tim2rst, arr_l, arr_h),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
opm! {
    TIM15: (apbenr2, apbrstr2, tim15, tim15en, tim15rst, arr),
}
