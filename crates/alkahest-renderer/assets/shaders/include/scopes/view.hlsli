cbuffer scope_view : register(b12) {
    float4x4 world_to_projective	: packoffset(c0);
    float4x4 camera_to_world		: packoffset(c4);

#if defined(STAGE_VS) || defined(STAGE_GS)
    float4 target		            : packoffset(c8);
    float4 view_miscellaneous		: packoffset(c9);
    float4 view_unk20               : packoffset(c10);
    float4x4 camera_to_projective   : packoffset(c11);
#else
    float4x4 target_pixel_to_camera	: packoffset(c8);
    float4 target		            : packoffset(c12);
    float4 view_miscellaneous		: packoffset(c13);
#endif
}; // cbuffer scope_view

 #define  camera_position		        (transpose(camera_to_world)[3].xyz)
 #define  camera_backward		        (transpose(camera_to_world)[2].xyz)
 #define  camera_up			            (transpose(camera_to_world)[1].xyz)
 #define  camera_right		            (transpose(camera_to_world)[0].xyz)
 #define  camera_forward		        (-transpose(camera_to_world)[2].xyz)
 #define  camera_down			        (-transpose(camera_to_world)[1].xyz)
 #define  camera_left			        (-transpose(camera_to_world)[0].xyz)
 #define  target_width		            (target.x)
 #define  target_height		            (target.y)
 #define  target_resolution	            (target.xy)
 #define  inverse_target_resolution	    (target.zw)
 #define  maximum_depth_pre_projection	(view_miscellaneous.x)
 #define  view_is_first_person			(view_miscellaneous.y)