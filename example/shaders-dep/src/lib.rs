#![cfg_attr(not(feature = "native"), no_std)]

use glam::Vec4;

#[derive(Debug, wgsl_gpu::Arguments)]
pub struct FragmentOutput {
	#[wgsl_gpu(location = 0)]
	pub color: Vec4,
}
