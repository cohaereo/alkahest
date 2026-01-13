use glam::{FloatExt, Vec4};

pub struct ExposureColumn {
    pub log_sum: f32,
    pub linear_sum: f32,
    pub weight_sum: f32,
}

pub struct ExposureResult {
    pub exposure_scale: f32,
    pub exposure_illum_relative: f32,
    pub scene_luminance: f32,
}

#[derive(Clone)]
pub struct AutoExposureConfig {
    /// Target middle grey value (e.g., 0.18).
    pub target_luminance: f32,
    pub min_luminance: f32,
    pub max_luminance: f32,

    /// Speed when adapting to a brighter scene (exposure going down)
    pub speed_dark_to_light: f32,

    /// Speed when adapting to a darker scene (exposure going up)
    pub speed_light_to_dark: f32,
}

impl Default for AutoExposureConfig {
    fn default() -> Self {
        Self {
            target_luminance: 0.0050,
            min_luminance: 0.0001,
            max_luminance: 65000.0,
            speed_dark_to_light: 2.0, // Fast reaction to bright areas
            speed_light_to_dark: 1.0, // Slow reaction to dark areas
        }
    }
}

/// Holds the state of the camera's exposure.
/// You should keep one instance of this per camera.
#[derive(Clone)]
pub struct AutoExposureSystem {
    pub config: AutoExposureConfig,

    // Current smoothed values applied to the frame
    pub current_exposure_scale: f32,
    pub current_illum_relative: f32,
}

impl AutoExposureSystem {
    pub fn new(config: AutoExposureConfig) -> Self {
        Self {
            config,
            // Initialize with a safe default (assuming 1.0 scale)
            current_exposure_scale: 0.050,
            current_illum_relative: 0.50,
        }
    }

    /// Feeds raw GPU columns into the system and returns the smoothed result for this frame.
    ///
    /// `raw_columns`: Flat array of floats representing ExposureColumn data from the GPU. Must be a multiple of 4.
    pub fn update_from_raw(&mut self, raw_columns: &[Vec4], delta_time: f32) -> ExposureResult {
        let mut columns = Vec::with_capacity(raw_columns.len());
        for col in raw_columns {
            columns.push(ExposureColumn {
                log_sum: col.x,
                linear_sum: col.y,
                weight_sum: col.z,
            });
        }

        self.update(&columns, delta_time)
    }

    /// Feeds raw GPU columns into the system and returns the smoothed result for this frame.
    /// `delta_time`: Time in seconds since the last frame.
    pub fn update(&mut self, columns: &[ExposureColumn], delta_time: f32) -> ExposureResult {
        let target = self.calculate_instant_target(columns);

        let is_adapting_to_light = target.exposure_scale < self.current_exposure_scale;

        let speed = if is_adapting_to_light {
            self.config.speed_dark_to_light
        } else {
            self.config.speed_light_to_dark
        };

        let blend_factor = 1.0 - (-speed * delta_time).exp();

        self.current_exposure_scale = self
            .current_exposure_scale
            .lerp(target.exposure_scale, blend_factor);

        self.current_illum_relative = self
            .current_illum_relative
            .lerp(target.exposure_illum_relative, blend_factor);

        ExposureResult {
            exposure_scale: self.current_exposure_scale,
            exposure_illum_relative: self.current_illum_relative,
            scene_luminance: target.scene_luminance, // Pass through raw luminance for debug info
        }
    }

    fn calculate_instant_target(&self, columns: &[ExposureColumn]) -> ExposureResult {
        let mut total_log_sum = 0.0;
        let mut total_lin_sum = 0.0;
        let mut total_weight = 0.0;

        for col in columns {
            total_log_sum += col.log_sum;
            total_lin_sum += col.linear_sum;
            total_weight += col.weight_sum;
        }

        if total_weight <= f32::EPSILON {
            return ExposureResult {
                exposure_scale: self.current_exposure_scale,
                exposure_illum_relative: self.current_illum_relative,
                scene_luminance: self.config.target_luminance,
            };
        }

        let avg_log_lum = total_log_sum / total_weight;
        let avg_lin_lum = total_lin_sum / total_weight;

        let scene_luminance_geo = avg_log_lum.exp2();
        let clamped_luminance =
            scene_luminance_geo.clamp(self.config.min_luminance, self.config.max_luminance);

        let target_scale = self.config.target_luminance / clamped_luminance;

        let target_illum = avg_lin_lum.clamp(0.0, 1.0);

        ExposureResult {
            exposure_scale: target_scale,
            exposure_illum_relative: target_illum,
            scene_luminance: clamped_luminance,
        }
    }
}

impl Default for AutoExposureSystem {
    fn default() -> Self {
        Self::new(AutoExposureConfig::default())
    }
}
