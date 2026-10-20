//! Portable maths: the same bits on every machine.
//!
//! Rust's `sin`, `exp`, `ln` and friends call each system's maths library, which can differ in
//! the last bit between macOS and Linux. In a forest one such bit can flip a decision (did it hear
//! that call?) and from there two renders go separate ways. Everything in kanopy-core that feeds
//! the forest or the sound uses these instead (pure Rust, from the `libm` crate). Square roots and
//! basic arithmetic are exact everywhere already.

pub trait Pm: Sized {
    fn p_sin(self) -> Self;
    fn p_cos(self) -> Self;
    fn p_tan(self) -> Self;
    fn p_exp(self) -> Self;
    fn p_exp2(self) -> Self;
    fn p_ln(self) -> Self;
    fn p_log10(self) -> Self;
    fn p_log2(self) -> Self;
    fn p_powf(self, e: Self) -> Self;
    fn p_hypot(self, o: Self) -> Self;
}

impl Pm for f64 {
    fn p_sin(self) -> f64 {
        libm::sin(self)
    }
    fn p_cos(self) -> f64 {
        libm::cos(self)
    }
    fn p_tan(self) -> f64 {
        libm::tan(self)
    }
    fn p_exp(self) -> f64 {
        libm::exp(self)
    }
    fn p_exp2(self) -> f64 {
        libm::exp2(self)
    }
    fn p_ln(self) -> f64 {
        libm::log(self)
    }
    fn p_log10(self) -> f64 {
        libm::log10(self)
    }
    fn p_log2(self) -> f64 {
        libm::log2(self)
    }
    fn p_powf(self, e: f64) -> f64 {
        libm::pow(self, e)
    }
    fn p_hypot(self, o: f64) -> f64 {
        libm::hypot(self, o)
    }
}

impl Pm for f32 {
    fn p_sin(self) -> f32 {
        libm::sinf(self)
    }
    fn p_cos(self) -> f32 {
        libm::cosf(self)
    }
    fn p_tan(self) -> f32 {
        libm::tanf(self)
    }
    fn p_exp(self) -> f32 {
        libm::expf(self)
    }
    fn p_exp2(self) -> f32 {
        libm::exp2f(self)
    }
    fn p_ln(self) -> f32 {
        libm::logf(self)
    }
    fn p_log10(self) -> f32 {
        libm::log10f(self)
    }
    fn p_log2(self) -> f32 {
        libm::log2f(self)
    }
    fn p_powf(self, e: f32) -> f32 {
        libm::powf(self, e)
    }
    fn p_hypot(self, o: f32) -> f32 {
        libm::hypotf(self, o)
    }
}
