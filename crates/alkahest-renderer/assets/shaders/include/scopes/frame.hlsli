SamplerState def_point_clamp : register(s0);
cbuffer scope_frame : register(b13)
{
    float4 time		            : packoffset(c0);
    float4 exposure		        : packoffset(c1);
    float4 random_seed_scales	: packoffset(c2);
    float4 overrides		    : packoffset(c3);
} // cbuffer scope_frame

#define  game_time			(time.x)
#define  render_time		(time.y)
#define  delta_game_time	(time.z)
#define  exposure_time		(time.w)

#define  exposure_scale	    (exposure.x)

// Exposure_scale_for_shading is equal to exposure_scale * 4 on xenon, and exposure_scale on other platforms
#define  exposure_illum_relative_glow	(exposure.y)
#define  exposure_scale_for_shading	    (exposure.z)
#define  exposure_illum_relative		(exposure.w)