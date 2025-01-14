use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::resources::{Buffer, BufferShared, ImageView, ImageViewShared};
use crate::video::{VideoSessionParameters, VideoSessionParametersShared};
use ash::vk::native::{
    StdVideoDecodeH264PictureInfo, StdVideoDecodeH264PictureInfoFlags, StdVideoDecodeH264ReferenceInfo,
    StdVideoDecodeH264ReferenceInfoFlags,
};
use ash::vk::{
    AccessFlags2, BufferMemoryBarrier2, DependencyInfoKHR, Extent2D, ImageAspectFlags, ImageLayout, ImageMemoryBarrier2,
    ImageSubresourceRange, PipelineStageFlags2, VideoBeginCodingInfoKHR, VideoCodingControlFlagsKHR, VideoCodingControlInfoKHR,
    VideoDecodeH264DpbSlotInfoKHR, VideoDecodeH264PictureInfoKHR, VideoDecodeInfoKHR, VideoEndCodingInfoKHR, VideoPictureResourceInfoKHR,
    VideoReferenceSlotInfoKHR, QUEUE_FAMILY_IGNORED,
};
use std::rc::Rc;
use std::sync::Arc;

/// Specifies which part of a buffer to decode.
#[derive(Copy, Clone)]
pub struct DecodeInfo {
    offset: u64,
    size: u64,
}

impl DecodeInfo {
    pub fn new(offset: u64, size: u64) -> Self {
        DecodeInfo { offset, size }
    }
}

/// Decode a H.264 video frame.
pub struct DecodeH264 {
    shared_parameters: Arc<VideoSessionParametersShared>,
    shared_buffer: Arc<BufferShared>,
    shared_image_view: Rc<ImageViewShared>,
    shared_ref_view: Rc<ImageViewShared>,
    decode_info: DecodeInfo,
}

impl DecodeH264 {
    pub fn new(
        buffer: &Buffer,
        video_session_parameters: &VideoSessionParameters,
        target_view: &ImageView,
        ref_view: &ImageView,
        decode_info: &DecodeInfo,
    ) -> Self {
        Self {
            shared_parameters: video_session_parameters.shared(),
            shared_buffer: buffer.shared(),
            shared_image_view: target_view.shared(),
            shared_ref_view: ref_view.shared(),
            decode_info: *decode_info,
        }
    }
}

impl AddToCommandBuffer for DecodeH264 {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error> {
        let shared_video_session = self.shared_parameters.video_session();

        let native_buffer_h264 = self.shared_buffer.native();
        let native_device = shared_video_session.device().native();
        let native_queue_fns = shared_video_session.queue_fns();
        let native_decode_fns = shared_video_session.decode_fns();
        let native_command_buffer = builder.native_command_buffer();
        let native_view_dst = self.shared_image_view.native();
        let native_view_ref = self.shared_ref_view.native();
        let native_image_dst = self.shared_image_view.image().native();
        let native_image_ref = self.shared_ref_view.image().native();
        let native_video_session = shared_video_session.native();
        let native_video_session_parameters = self.shared_parameters.native();

        let image_info = self.shared_image_view.image().info();
        let image_extent = image_info.get_extent();
        let extent = Extent2D::default().width(image_extent.width).height(image_extent.height);

        let picture_resource_dst = VideoPictureResourceInfoKHR::default()
            .coded_extent(extent)
            .image_view_binding(native_view_dst);

        let picture_resource_ref = VideoPictureResourceInfoKHR::default()
            .coded_extent(extent)
            .image_view_binding(native_view_ref);

        let mut f = StdVideoDecodeH264ReferenceInfoFlags {
            _bitfield_align_1: [],
            _bitfield_1: Default::default(),
            __bindgen_padding_0: Default::default(),
        };
        f.set_used_for_long_term_reference(1);

        let s = StdVideoDecodeH264ReferenceInfo {
            flags: f,
            FrameNum: 0,
            reserved: 0,
            PicOrderCnt: [0, 0],
        };

        let mut video_decode_h264_dpb_slot_info = VideoDecodeH264DpbSlotInfoKHR::default().std_reference_info(&s);

        let video_reference_slot = VideoReferenceSlotInfoKHR::default()
            .push_next(&mut video_decode_h264_dpb_slot_info)
            .slot_index(0)
            .picture_resource(&picture_resource_dst);

        let begin_coding_info = VideoBeginCodingInfoKHR::default()
            .video_session(native_video_session)
            .video_session_parameters(native_video_session_parameters);

        let end_coding_info = VideoEndCodingInfoKHR::default();

        let mut stdflags = StdVideoDecodeH264PictureInfoFlags {
            _bitfield_align_1: Default::default(),
            _bitfield_1: Default::default(),
            __bindgen_padding_0: Default::default(),
        };

        stdflags.set_is_intra(1);
        stdflags.set_is_reference(1);

        let std = StdVideoDecodeH264PictureInfo {
            flags: stdflags,
            seq_parameter_set_id: 0,
            pic_parameter_set_id: 0,
            reserved1: 0,
            reserved2: 0,
            frame_num: 0,
            idr_pic_id: 0,
            PicOrderCnt: [0, 0], // TODO: ???
        };

        let video_coding_control = VideoCodingControlInfoKHR::default().flags(VideoCodingControlFlagsKHR::RESET);
        let mut video_decode_info_h264 = VideoDecodeH264PictureInfoKHR::default().std_picture_info(&std).slice_offsets(&[0]);

        let video_decode_info = VideoDecodeInfoKHR::default()
            .push_next(&mut video_decode_info_h264)
            .src_buffer(native_buffer_h264)
            .src_buffer_offset(self.decode_info.offset)
            .src_buffer_range(self.decode_info.size)
            // .src_buffer_range(2736)
            .dst_picture_resource(picture_resource_dst)
            .setup_reference_slot(&video_reference_slot);

        unsafe {
            let ssr = ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);

            let image_barrier_dst = ImageMemoryBarrier2::default()
                .src_stage_mask(PipelineStageFlags2::NONE)
                .src_access_mask(AccessFlags2::NONE)
                .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                .old_layout(ImageLayout::UNDEFINED)
                .dst_stage_mask(PipelineStageFlags2::VIDEO_DECODE_KHR)
                .dst_access_mask(AccessFlags2::VIDEO_DECODE_WRITE_KHR)
                .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
                .new_layout(ImageLayout::VIDEO_DECODE_DPB_KHR)
                .image(native_image_dst)
                .subresource_range(ssr);

            let image_release_dst = ImageMemoryBarrier2::default()
                .src_stage_mask(PipelineStageFlags2::VIDEO_DECODE_KHR)
                .src_access_mask(AccessFlags2::VIDEO_DECODE_WRITE_KHR)
                .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                .old_layout(ImageLayout::VIDEO_DECODE_DPB_KHR)
                .dst_stage_mask(PipelineStageFlags2::BOTTOM_OF_PIPE)
                .dst_access_mask(AccessFlags2::NONE_KHR)
                .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
                .new_layout(ImageLayout::GENERAL)
                .image(native_image_dst)
                .subresource_range(ssr);

            let buffer_barrier = BufferMemoryBarrier2::default()
                .src_stage_mask(PipelineStageFlags2::HOST)
                .src_access_mask(AccessFlags2::HOST_WRITE)
                .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                .dst_stage_mask(PipelineStageFlags2::VIDEO_DECODE_KHR)
                .dst_access_mask(AccessFlags2::VIDEO_DECODE_READ_KHR)
                .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
                .buffer(native_buffer_h264)
                .size(256 * 16);

            let buffer_barrier_release = BufferMemoryBarrier2::default()
                .src_stage_mask(PipelineStageFlags2::VIDEO_DECODE_KHR)
                .src_access_mask(AccessFlags2::VIDEO_DECODE_READ_KHR)
                .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                .dst_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
                .dst_access_mask(AccessFlags2::NONE)
                .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
                .buffer(native_buffer_h264)
                .size(256 * 16);

            let buffer_barriers = &[buffer_barrier];
            let buffer_barriers_release = &[buffer_barrier_release];
            let image_barriers = &[image_barrier_dst];
            let image_barriers_release = &[image_release_dst];

            let dependency_info = DependencyInfoKHR::default()
                .buffer_memory_barriers(buffer_barriers)
                .image_memory_barriers(image_barriers);

            let dependency_info_release = DependencyInfoKHR::default()
                .buffer_memory_barriers(buffer_barriers_release)
                .image_memory_barriers(image_barriers_release);

            native_device.cmd_pipeline_barrier2(native_command_buffer, &dependency_info);
            (native_queue_fns.cmd_begin_video_coding_khr)(native_command_buffer, &begin_coding_info);
            (native_queue_fns.cmd_control_video_coding_khr)(native_command_buffer, &video_coding_control);
            (native_decode_fns.cmd_decode_video_khr)(native_command_buffer, &video_decode_info);
            (native_queue_fns.cmd_end_video_coding_khr)(native_command_buffer, &end_coding_info);
            native_device.cmd_pipeline_barrier2(native_command_buffer, &dependency_info_release);

            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error;
    use crate::error::{Error, Variant};
    use crate::instance::{Instance, InstanceInfo};
    use crate::ops::decodeh264::DecodeInfo;
    use crate::ops::{AddToCommandBuffer, CopyImage2Buffer, DecodeH264};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    use crate::resources::{Buffer, BufferInfo, Image, ImageInfo, ImageView, ImageViewInfo};
    use crate::video::h264::H264StreamInspector;
    use crate::video::{VideoSession, VideoSessionParameters};
    use ash::vk::{
        Extent3D, Format, ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageViewType, SampleCountFlags,
    };

    #[test]
    #[cfg(not(miri))]
    fn decode_h264() -> Result<(), Error> {
        let h264_data = include_bytes!("../../tests/videos/multi_512x512.h264");

        let stream_inspector = H264StreamInspector::new();
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let image_dst_info = ImageInfo::new()
            .format(Format::G8_B8R8_2PLANE_420_UNORM)
            .samples(SampleCountFlags::TYPE_1)
            .usage(
                ImageUsageFlags::TRANSFER_SRC
                    | ImageUsageFlags::TRANSFER_DST
                    | ImageUsageFlags::VIDEO_DECODE_DST_KHR
                    | ImageUsageFlags::VIDEO_DECODE_DPB_KHR,
            )
            .mip_levels(1)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .tiling(ImageTiling::OPTIMAL)
            .layout(ImageLayout::UNDEFINED)
            .extent(Extent3D::default().width(512).height(512).depth(1));

        let image_dst = Image::new_video_target(&device, &image_dst_info, &stream_inspector)?;
        let image_ref = Image::new_video_target(&device, &image_dst_info, &stream_inspector)?;
        let heap_image = image_dst.memory_requirement().any_heap();
        let allocation_image_dst = Allocation::new(&device, 512 * 512 * 4, heap_image)?;
        let allocation_image_ref = Allocation::new(&device, 512 * 512 * 4, heap_image)?;
        let image_dst = image_dst.bind(&allocation_image_dst)?;
        let image_ref = image_ref.bind(&allocation_image_ref)?;

        let image_view_dst_info = ImageViewInfo::new()
            .aspect_mask(ImageAspectFlags::COLOR)
            .format(Format::G8_B8R8_2PLANE_420_UNORM)
            .image_view_type(ImageViewType::TYPE_2D)
            .layer_count(1)
            .level_count(1);
        let image_view_dst = ImageView::new(&image_dst, &image_view_dst_info)?;
        let image_view_ref = ImageView::new(&image_ref, &image_view_dst_info)?;
        let queue_video_decode = physical_device
            .queue_family_infos()
            .any_decode()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;
        let queue_compute = physical_device
            .queue_family_infos()
            .any_compute()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;
        let queue = Queue::new(&device, queue_video_decode, 0)?;
        let queue_copy = Queue::new(&device, queue_compute, 0)?;
        let command_buffer = CommandBuffer::new(&device, queue_video_decode)?;
        let command_buffer_copy = CommandBuffer::new(&device, queue_compute)?;

        // TODO: WHY THIS +256 needed for video buffers?
        let memory_host = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        // let memory_device = physical_device
        //     .heap_infos()
        //     .any_device_local()
        //     .ok_or_else(|| error!(Variant::HeapNotFound))?;

        let allocation_h264 = Allocation::new(&device, 1024 * 1024 * 4 + 256, memory_host)?;
        let buffer_info_h264 = BufferInfo::new().size(1024 * 1024 * 4);
        let buffer_h264 = Buffer::new_video_decode(&allocation_h264, &buffer_info_h264, &stream_inspector)?;

        buffer_h264.upload(&h264_data[0..])?;

        let allocation_output = Allocation::new(&device, 512 * 512 * 4, memory_host)?;
        let buffer_info_output = BufferInfo::new().size(512 * 512 * 4);
        let buffer_output = Buffer::new(&allocation_output, &buffer_info_output)?;

        let video_session = VideoSession::new(&device, &stream_inspector)?;
        let video_session_parameters = VideoSessionParameters::new(&video_session, &stream_inspector)?;
        let decode_info = DecodeInfo::new(0, 16 * 256);

        let decode = DecodeH264::new(
            &buffer_h264,
            &video_session_parameters,
            &image_view_dst,
            &image_view_ref,
            &decode_info,
        );
        let copy = CopyImage2Buffer::new(&image_dst, &buffer_output, ImageAspectFlags::PLANE_0);

        queue.build_and_submit(&command_buffer, |x| {
            decode.run_in(x)?;
            Ok(())
        })?;

        // Copy image2buffer has to run on a queue with compute or graphics capabilities, which
        // the video decode queue doesn't have on my graphics card
        queue_copy.build_and_submit(&command_buffer_copy, |x| {
            copy.run_in(x)?;
            Ok(())
        })?;

        let mut data_out = [0u8; 512 * 512 * 4];
        buffer_output.download_into(&mut data_out)?;

        assert_eq!(data_out[0], 108);
        assert_eq!(data_out[1], 108);
        assert_eq!(data_out[2], 108);
        assert_eq!(data_out[3], 108);

        Ok(())
    }
}
