// use ash::vk::{Extent3D, Format, ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageViewType, SampleCountFlags};
// use vulkan_video::ops::{AddToCommandBuffer, CopyImage2Buffer, DecodeH264, DecodeInfo, FillBuffer};
// use vulkan_video::resources::{Buffer, BufferInfo, Image, ImageInfo, ImageView, ImageViewInfo};
// use vulkan_video::video::h264::H264StreamInspector;
// use vulkan_video::video::{nal_units, VideoSession, VideoSessionParameters};
// use vulkan_video::{error, Allocation, CommandBuffer, Device, Error, Instance, InstanceInfo, PhysicalDevice, Queue, Variant};
//
// #[test]
// #[cfg(not(miri))]
// fn decode_multiple_h264_frames() -> Result<(), Error> {
//     let h264_data = include_bytes!("videos/multi_512x512.h264");
//
//     let stream_inspector = H264StreamInspector::new();
//     let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
//     let instance = Instance::new(&instance_info)?;
//     let physical_device = PhysicalDevice::new_any(&instance)?;
//     let device = Device::new(&physical_device)?;
//     let image_dst_info = ImageInfo::new()
//         .format(Format::G8_B8R8_2PLANE_420_UNORM)
//         .samples(SampleCountFlags::TYPE_1)
//         .usage(
//             ImageUsageFlags::TRANSFER_SRC
//                 | ImageUsageFlags::TRANSFER_DST
//                 | ImageUsageFlags::VIDEO_DECODE_DST_KHR
//                 | ImageUsageFlags::VIDEO_DECODE_DPB_KHR,
//         )
//         .mip_levels(1)
//         .array_layers(1)
//         .image_type(ImageType::TYPE_2D)
//         .tiling(ImageTiling::OPTIMAL)
//         .layout(ImageLayout::UNDEFINED)
//         .extent(Extent3D::default().width(512).height(512).depth(1));
//
//     let image_dst = Image::new_video_target(&device, &image_dst_info, &stream_inspector)?;
//     let heap_image = image_dst.memory_requirement().any_heap();
//     let allocation_image_dst = Allocation::new(&device, 512 * 512 * 4, heap_image)?;
//     let image_dst = image_dst.bind(&allocation_image_dst)?;
//
//     let image_view_dst_info = ImageViewInfo::new()
//         .aspect_mask(ImageAspectFlags::COLOR)
//         .format(Format::G8_B8R8_2PLANE_420_UNORM)
//         .image_view_type(ImageViewType::TYPE_2D)
//         .layer_count(1)
//         .level_count(1);
//     let image_view_dst = ImageView::new(&image_dst, &image_view_dst_info)?;
//     let queue_video_decode = physical_device
//         .queue_family_infos()
//         .any_decode()
//         .ok_or_else(|| error!(Variant::QueueNotFound))?;
//     let queue = Queue::new(&device, queue_video_decode, 0)?;
//     let command_buffer = CommandBuffer::new(&device, queue_video_decode)?;
//
//     // TODO: WHY THIS +256 needed for video buffers?
//     let memory_host = physical_device
//         .heap_infos()
//         .any_host_visible()
//         .ok_or_else(|| error!(Variant::HeapNotFound))?;
//
//     let allocation_h264 = Allocation::new(&device, 1024 * 1024 * 4 + 256, memory_host)?;
//     let buffer_info_h264 = BufferInfo::new().size(1024 * 1024 * 4);
//     let buffer_h264 = Buffer::new_video_decode(&allocation_h264, &buffer_info_h264, &stream_inspector)?;
//
//     buffer_h264.upload(h264_data)?;
//
//     let allocation_output = Allocation::new(&device, 512 * 512 * 4, memory_host)?;
//     let buffer_info_output = BufferInfo::new().size(512 * 512 * 4);
//     let buffer_output = Buffer::new(&allocation_output, &buffer_info_output)?;
//
//     let mut offset = 0;
//
//     for nal in nal_units(h264_data) {
//         let video_session = VideoSession::new(&device, &stream_inspector)?;
//         let video_session_parameters = VideoSessionParameters::new(&video_session, &stream_inspector)?;
//
//         let decode_info = DecodeInfo::new(offset, nal.len() as u64);
//
//         let fill = FillBuffer::new(&buffer_output, 0);
//         let decode = DecodeH264::new(&buffer_h264, &video_session_parameters, &image_view_dst, &decode_info);
//         let copy = CopyImage2Buffer::new(&image_dst, &buffer_output, ImageAspectFlags::PLANE_0);
//
//         queue.build_and_submit(&command_buffer, |x| {
//             fill.run_in(x)?;
//             decode.run_in(x)?;
//             copy.run_in(x)?;
//             Ok(())
//         })?;
//
//         let mut data_out = [0u8; 512 * 512 * 4];
//         buffer_output.download_into(&mut data_out)?;
//
//         offset += nal.len() as u64;
//
//         dbg!(&data_out[0..10]);
//     }
//
//     Ok(())
// }
