#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::mpsc::channel;
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
use half::f16;
#[cfg(windows)]
use image::RgbaImage;
#[cfg(windows)]
use scopeguard::guard;
#[cfg(windows)]
use tauri::Runtime;
#[cfg(windows)]
use windows::{
    core::{factory, IInspectable, Interface, HRESULT},
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SDR_WHITE_LEVEL, DISPLAYCONFIG_SOURCE_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    },
    Win32::{
        Foundation::{POINT, WIN32_ERROR},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource,
                ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_READ,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
                D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
            },
            Dxgi::IDXGIDevice,
            Gdi::{
                GetMonitorInfoW, MonitorFromPoint, HMONITOR, MONITORINFOEXW,
                MONITOR_DEFAULTTONEAREST,
            },
        },
        System::WinRT::{
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
            Graphics::Capture::IGraphicsCaptureItemInterop,
        },
    },
};

#[cfg(windows)]
use crate::domain::{CaptureError, ResolvedCaptureArea};

#[cfg(windows)]
fn create_d3d_device() -> Result<ID3D11Device, CaptureError> {
    unsafe {
        let mut d3d_device = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            Default::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut d3d_device),
            None,
            None,
        )
        .map_err(|error| CaptureError::new("d3d11_create_failed", error.to_string()))?;

        d3d_device.ok_or_else(|| CaptureError::new("d3d11_create_failed", "D3D11 device is null."))
    }
}

#[cfg(windows)]
fn create_capture_item(h_monitor: HMONITOR) -> Result<GraphicsCaptureItem, CaptureError> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .map_err(|error| CaptureError::new("capture_interop_failed", error.to_string()))?;

    unsafe {
        interop
            .CreateForMonitor::<GraphicsCaptureItem>(h_monitor)
            .map_err(|error| CaptureError::new("capture_item_failed", error.to_string()))
    }
}

#[cfg(windows)]
fn sc_rgb_to_srgb_u8(value: f32) -> u8 {
    let linear = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };

    let srgb = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };

    (srgb.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[cfg(windows)]
fn reinhard(x: f32) -> f32 {
    x / (1.0 + x)
}

#[cfg(windows)]
fn get_sdr_white_level_scale(h_monitor: HMONITOR) -> Result<f32, CaptureError> {
    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    unsafe {
        if !GetMonitorInfoW(h_monitor, &mut monitor_info.monitorInfo as *mut _).as_bool() {
            return Err(CaptureError::new(
                "monitor_info_failed",
                "GetMonitorInfoW failed.",
            ));
        }

        let mut number_of_paths = 0;
        let mut number_of_modes = 0;
        let _ = GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            &mut number_of_modes,
        );
        if number_of_paths == 0 && number_of_modes == 0 {
            return Ok(1.0);
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); number_of_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); number_of_modes as usize];

        let query_status = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            paths.as_mut_ptr(),
            &mut number_of_modes,
            modes.as_mut_ptr(),
            None,
        );
        if query_status != WIN32_ERROR(0) {
            return Ok(1.0);
        }

        for path in paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: path.sourceInfo.adapterId,
                    id: path.sourceInfo.id,
                },
                ..Default::default()
            };

            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }

            if source.viewGdiDeviceName != monitor_info.szDevice {
                continue;
            }

            let mut sdr_white = DISPLAYCONFIG_SDR_WHITE_LEVEL {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL,
                    size: std::mem::size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };

            if DisplayConfigGetDeviceInfo(&mut sdr_white.header) != 0 {
                break;
            }

            // Windows 官方标准换算
            let actual_nits = sdr_white.SDRWhiteLevel as f32 * 80.0 / 1000.0;
            let scale = (actual_nits / 800.0).clamp(0.2, 5.0);
            return Ok(scale);
        }
    }

    Ok(1.0)
}

#[cfg(windows)]
fn ensure_hdr_disabled(h_monitor: HMONITOR) -> Result<(), CaptureError> {
    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    unsafe {
        if !GetMonitorInfoW(h_monitor, &mut monitor_info.monitorInfo as *mut _).as_bool() {
            return Err(CaptureError::new(
                "monitor_info_failed",
                "GetMonitorInfoW failed.",
            ));
        }

        let mut number_of_paths = 0;
        let mut number_of_modes = 0;
        let _ = GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            &mut number_of_modes,
        );
        if number_of_paths == 0 && number_of_modes == 0 {
            return Ok(());
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); number_of_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); number_of_modes as usize];

        let query_status = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            paths.as_mut_ptr(),
            &mut number_of_modes,
            modes.as_mut_ptr(),
            None,
        );
        if query_status != WIN32_ERROR(0) {
            return Ok(());
        }

        for path in paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: path.sourceInfo.adapterId,
                    id: path.sourceInfo.id,
                },
                ..Default::default()
            };

            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }

            if source.viewGdiDeviceName != monitor_info.szDevice {
                continue;
            }

            let mut advanced_color = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                    size: std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };

            if DisplayConfigGetDeviceInfo(&mut advanced_color.header) != 0 {
                break;
            }

            let flags = advanced_color.Anonymous.value;
            let advanced_color_active = (flags & 0x1) != 0;

            if advanced_color_active {
                return Err(CaptureError::new(
                    "hdr_unsupported",
                    "当前显示器开启了 HDR / Advanced Color，当前版本不支持截图。请先关闭 HDR。",
                ));
            }

            return Ok(());
        }
    }

    Ok(())
}

#[cfg(windows)]
fn texture_to_rgba_image(
    d3d_device: &ID3D11Device,
    d3d_context: &ID3D11DeviceContext,
    source_texture: &ID3D11Texture2D,
    sdr_white_scale: f32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, CaptureError> {
    unsafe {
        let mut src_desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut src_desc);

        if x + width > src_desc.Width || y + height > src_desc.Height {
            return Err(CaptureError::new(
                "capture_out_of_bounds",
                "ROI is outside the captured texture.",
            ));
        }

        let staging_texture = {
            let mut staging_desc = src_desc;
            staging_desc.Width = width;
            staging_desc.Height = height;
            staging_desc.BindFlags = 0;
            staging_desc.MiscFlags = 0;
            staging_desc.Usage = D3D11_USAGE_STAGING;
            staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

            let mut staging = None;
            d3d_device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging))
                .map_err(|error| CaptureError::new("staging_texture_failed", error.to_string()))?;
            staging.ok_or_else(|| {
                CaptureError::new("staging_texture_failed", "Staging texture is null.")
            })?
        };

        let region = D3D11_BOX {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
            front: 0,
            back: 1,
        };

        d3d_context.CopySubresourceRegion(
            Some(
                &staging_texture
                    .cast()
                    .map_err(|error| CaptureError::new("staging_cast_failed", error.to_string()))?,
            ),
            0,
            0,
            0,
            0,
            Some(
                &source_texture
                    .cast()
                    .map_err(|error| CaptureError::new("source_cast_failed", error.to_string()))?,
            ),
            0,
            Some(&region),
        );

        let resource: ID3D11Resource = staging_texture
            .cast()
            .map_err(|error| CaptureError::new("resource_cast_failed", error.to_string()))?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();

        d3d_context
            .Map(Some(&resource), 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .map_err(|error| CaptureError::new("texture_map_failed", error.to_string()))?;

        let _unmap_guard = guard((), |_| {
            d3d_context.Unmap(Some(&resource), 0);
        });

        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let src_ptr = mapped.pData as *const u8;
        const MAX_HDR: f32 = 2.5;

        for row in 0..height {
            let row_base = src_ptr.add((row * mapped.RowPitch) as usize);
            for col in 0..width as usize {
                let px_off = col * 8;
                let px_ptr = row_base.add(px_off);

                // 读取 f16 HDR 像素
                let r = f16::from_bits(u16::from_le_bytes([*px_ptr, *px_ptr.add(1)])).to_f32();
                let g =
                    f16::from_bits(u16::from_le_bytes([*px_ptr.add(2), *px_ptr.add(3)])).to_f32();
                let b =
                    f16::from_bits(u16::from_le_bytes([*px_ptr.add(4), *px_ptr.add(5)])).to_f32();
                let a =
                    f16::from_bits(u16::from_le_bytes([*px_ptr.add(6), *px_ptr.add(7)])).to_f32();

                // 白电平校正 + 高光钳位
                let mut r = (r / sdr_white_scale).max(0.0).min(MAX_HDR);
                let mut g = (g / sdr_white_scale).max(0.0).min(MAX_HDR);
                let mut b = (b / sdr_white_scale).max(0.0).min(MAX_HDR);

                // 色调映射，消除雾感
                r = reinhard(r);
                g = reinhard(g);
                b = reinhard(b);

                let dst = (row as usize * width as usize + col) * 4;
                rgba[dst] = sc_rgb_to_srgb_u8(r);
                rgba[dst + 1] = sc_rgb_to_srgb_u8(g);
                rgba[dst + 2] = sc_rgb_to_srgb_u8(b);
                rgba[dst + 3] = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }

        RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| CaptureError::new("rgba_image_failed", "RgbaImage::from_raw failed."))
    }
}

#[cfg(windows)]
fn capture_hmonitor_region_hdr(
    h_monitor: HMONITOR,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, CaptureError> {
    ensure_hdr_disabled(h_monitor)?;
    let sdr_white_scale = get_sdr_white_level_scale(h_monitor)?;
    let d3d_device = create_d3d_device()?;
    let d3d_context = unsafe {
        d3d_device
            .GetImmediateContext()
            .map_err(|error| CaptureError::new("d3d11_context_failed", error.to_string()))?
    };
    let dxgi_device = d3d_device
        .cast::<IDXGIDevice>()
        .map_err(|error| CaptureError::new("dxgi_cast_failed", error.to_string()))?;
    let device = unsafe {
        CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)
            .map_err(|error| CaptureError::new("direct3d_device_failed", error.to_string()))?
            .cast::<IDirect3DDevice>()
            .map_err(|error| CaptureError::new("direct3d_device_cast_failed", error.to_string()))?
    };

    let item = create_capture_item(h_monitor)?;
    let item_size = item
        .Size()
        .map_err(|error| CaptureError::new("capture_item_size_failed", error.to_string()))?;

    let frame_pool = guard(
        Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::R16G16B16A16Float,
            1,
            item_size,
        )
        .map_err(|error| CaptureError::new("frame_pool_failed", error.to_string()))?,
        |value| {
            let _ = value.Close();
        },
    );

    let session = guard(
        frame_pool
            .CreateCaptureSession(&item)
            .map_err(|error| CaptureError::new("capture_session_failed", error.to_string()))?,
        |value| {
            let _ = value.Close();
        },
    );

    let (sender, receiver) = channel();

    frame_pool
        .FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().ok_or_else(|| {
                        windows::core::Error::new(
                            HRESULT(0x80004005u32 as i32),
                            "Frame pool is null",
                        )
                    })?;
                    let frame = guard(frame_pool.TryGetNextFrame()?, |value| {
                        let _ = value.Close();
                    });

                    let surface = frame.Surface()?;
                    let access = surface.cast::<IDirect3DDxgiInterfaceAccess>()?;
                    let source_texture = unsafe { access.GetInterface::<ID3D11Texture2D>()? };
                    let image = texture_to_rgba_image(
                        &d3d_device,
                        &d3d_context,
                        &source_texture,
                        sdr_white_scale,
                        x,
                        y,
                        width,
                        height,
                    )
                    .map_err(|error| {
                        windows::core::Error::new(HRESULT(0x80004005u32 as i32), &error.message)
                    })?;

                    let _ = sender.send(image);
                    Ok(())
                }
            }),
        )
        .map_err(|error| CaptureError::new("frame_arrived_failed", error.to_string()))?;

    let _ = session.SetIsBorderRequired(false);
    let _ = session.SetIsCursorCaptureEnabled(false);
    session
        .StartCapture()
        .map_err(|error| CaptureError::new("capture_start_failed", error.to_string()))?;

    receiver
        .recv_timeout(Duration::from_millis(3000))
        .map_err(|error| CaptureError::new("capture_timeout", error.to_string()))
}

#[cfg(windows)]
pub fn capture_area_hdr_to_file<R: Runtime>(
    app: &tauri::AppHandle<R>,
    resolved: &ResolvedCaptureArea,
) -> Result<crate::domain::CaptureResult, CaptureError> {
    let image = capture_hmonitor_region_hdr(
        resolved.monitor_handle,
        resolved.relative_x,
        resolved.relative_y,
        resolved.width,
        resolved.height,
    )?;

    super::capture_area::save_image_to_cache(app, image)
}

#[cfg(windows)]
pub fn capture_primary_monitor_hdr_to_file<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<crate::domain::CaptureResult, CaptureError> {
    let monitor = app
        .primary_monitor()
        .map_err(|error| CaptureError::new("monitor_query_failed", error.to_string()))?
        .ok_or_else(|| CaptureError::new("monitor_not_found", "No primary monitor found."))?;

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let h_monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: monitor_position.x + 1,
                y: monitor_position.y + 1,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };

    if h_monitor.0 == null_mut() {
        return Err(CaptureError::new(
            "monitor_not_found",
            "No monitor found for the primary display.",
        ));
    }

    ensure_hdr_disabled(h_monitor)?;

    let image =
        capture_hmonitor_region_hdr(h_monitor, 0, 0, monitor_size.width, monitor_size.height)?;

    super::capture_area::save_image_to_cache(app, image)
}
