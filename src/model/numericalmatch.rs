use super::{base::ImagePairClassifierPredictor, Predictor};
use crate::BootArgs;
use anyhow::Result;
use image::DynamicImage;

pub struct NumericalmatchPredictor(ImagePairClassifierPredictor);

impl NumericalmatchPredictor {
    /// Create a new instance of the NumericalmatchPredictor
    pub fn new(args: &BootArgs) -> Result<Self> {
        Ok(Self(ImagePairClassifierPredictor::new(
            "numericalmatch.onnx",
            args,
            false,
        )?))
    }
}

impl Predictor for NumericalmatchPredictor {
    fn predict(&self, image: DynamicImage) -> Result<i32> {
        self.0.predict(image)
    }
}