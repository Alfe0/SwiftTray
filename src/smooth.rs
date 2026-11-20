//! Lightweight anti-aliased painting using the GDI+ runtime included with Windows.
//! Draws shapes and scales native Windows HICONs with GDI+ while preserving their colors.
use std::{ffi::c_void, ptr};

type Gp = *mut c_void;

#[repr(C)]
struct GpPointF { x:f32, y:f32 }

#[repr(C)]
struct StartupInput {
    version: u32,
    debug_callback: usize,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const StartupInput, output: *mut c_void) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipCreateFromHDC(hdc: isize, graphics: *mut Gp) -> i32;
    fn GdipDeleteGraphics(graphics: Gp) -> i32;
    fn GdipSetSmoothingMode(graphics: Gp, mode: i32) -> i32;
    fn GdipSetPixelOffsetMode(graphics: Gp, mode: i32) -> i32;
    fn GdipSetInterpolationMode(graphics: Gp, mode: i32) -> i32;
    fn GdipCreateBitmapFromHICON(icon: isize, bitmap: *mut Gp) -> i32;
    // A memory-backed, premultiplied-alpha logo avoids the HICON -> GDI+
    // conversion that introduced colored outlines on the About screen.
    fn GdipCreateBitmapFromScan0(width: i32, height: i32, stride: i32,
        pixel_format: i32, scan0: *mut u8, bitmap: *mut Gp) -> i32;
    fn GdipGetImageWidth(image: Gp, width: *mut u32) -> i32;
    fn GdipGetImageHeight(image: Gp, height: *mut u32) -> i32;
    fn GdipDrawImageRectRectI(graphics: Gp, image: Gp,
        dest_x: i32, dest_y: i32, dest_w: i32, dest_h: i32,
        src_x: i32, src_y: i32, src_w: i32, src_h: i32, unit: i32,
        attributes: Gp, abort: *const c_void, callback: *const c_void) -> i32;
    fn GdipDisposeImage(image: Gp) -> i32;
    fn GdipCreateImageAttributes(attributes: *mut Gp) -> i32;
    fn GdipDisposeImageAttributes(attributes: Gp) -> i32;
    fn GdipSetImageAttributesColorMatrix(attributes: Gp, adjust_type: i32,
        enable: i32, matrix: *const f32, gray: *const f32, flags: i32) -> i32;
    fn GdipAddPathRectangle(path: Gp, x:f32, y:f32, w:f32, h:f32) -> i32;
    fn GdipFlush(graphics: Gp, intention: i32) -> i32;
    fn GdipCreateSolidFill(argb: u32, brush: *mut Gp) -> i32;
    fn GdipCreateLineBrush(start:*const GpPointF,end:*const GpPointF,
        first:u32,second:u32,wrap_mode:i32,brush:*mut Gp)->i32;
    fn GdipSetLinePresetBlend(brush: Gp, colors: *const u32,
        positions: *const f32, count: i32) -> i32;
    fn GdipDeleteBrush(brush: Gp) -> i32;
    fn GdipFillEllipse(graphics: Gp, brush: Gp, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipFillRectangle(graphics: Gp, brush: Gp, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipCreatePath(fill_mode: i32, path: *mut Gp) -> i32;
    fn GdipAddPathArc(path: Gp, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> i32;
    fn GdipClosePathFigure(path: Gp) -> i32;
    fn GdipFillPath(graphics: Gp, brush: Gp, path: Gp) -> i32;
    fn GdipDeletePath(path: Gp) -> i32;
}

pub fn start() -> Option<usize> {
    let mut token = 0usize;
    let input = StartupInput {
        version: 1,
        debug_callback: 0,
        suppress_background_thread: 0,
        suppress_external_codecs: 0,
    };
    let result = unsafe { GdiplusStartup(&mut token, &input, ptr::null_mut()) };
    (result == 0 && token != 0).then_some(token)
}

pub fn stop(token: usize) {
    if token != 0 { unsafe { GdiplusShutdown(token); } }
}

fn argb(color: u32) -> u32 {
    0xff00_0000 | ((color & 0xff) << 16) | (color & 0xff00) | ((color >> 16) & 0xff)
}

/// Decode each native HICON once per icon-cache entry, not 25 times a second.
/// The owning IconCache releases these bitmaps before GDI+ shuts down.
// Generated from the original SVG at build-time by tools/generate_icons.py.
// Premultiplied BGRA, top-to-bottom, 256x256; embedded in the one EXE.
const ABOUT_SIDE:usize = 256;
const ABOUT_BYTES:&[u8; ABOUT_SIDE * ABOUT_SIDE * 4] =
    include_bytes!("../assets/SwiftTray-about.pbgra");

pub fn about_logo_pixels() -> Vec<u8> { ABOUT_BYTES.to_vec() }

/// The caller owns the backing pixel allocation until `release_native_icon`.
/// GDI+ may retain the scan0 pointer: never drop/resize the Vec while in use.
pub fn prepare_about_logo(pixels: &mut [u8]) -> isize {
    const PIXEL_FORMAT_32BPP_PARGB:i32 = 0x000E200B;
    if pixels.len() != ABOUT_BYTES.len() { return 0; }
    let mut bitmap:Gp = ptr::null_mut();
    let result=unsafe {GdipCreateBitmapFromScan0(
        ABOUT_SIDE as i32,ABOUT_SIDE as i32,(ABOUT_SIDE*4) as i32,
        PIXEL_FORMAT_32BPP_PARGB,pixels.as_mut_ptr(),&mut bitmap)};
    if result == 0 && !bitmap.is_null() { bitmap as isize } else { 0 }
}

// Original GitHub icon-shaped artwork, rendered once from the outline SVG
// and embedded in the executable. It is a WHITE premultiplied-alpha mask so
// the draw routine can preserve the user's Windows/SwiftTray accent color.
const GITHUB_SIDE:usize = 64;
const GITHUB_BYTES:&[u8;GITHUB_SIDE*GITHUB_SIDE*4] =
    include_bytes!("../assets/GitHub-mark.pbgra");

pub fn github_mark_pixels() -> Vec<u8> { GITHUB_BYTES.to_vec() }

// Keep this Vec alive until the associated bitmap has been released: GDI+
// may retain scan0 instead of copying the pixels.
pub fn prepare_github_mark(pixels:&mut [u8]) -> isize {
    const PIXEL_FORMAT_32BPP_PARGB:i32 = 0x000E200B;
    if pixels.len()!=GITHUB_BYTES.len() {return 0;}
    let mut bitmap:Gp=ptr::null_mut();
    let status=unsafe {GdipCreateBitmapFromScan0(
        GITHUB_SIDE as i32,GITHUB_SIDE as i32,(GITHUB_SIDE*4) as i32,
        PIXEL_FORMAT_32BPP_PARGB,pixels.as_mut_ptr(),&mut bitmap)};
    if status==0 && !bitmap.is_null() {bitmap as isize}else{0}
}

pub fn prepare_native_icon(hicon:isize) -> isize {
    if hicon==0 {return 0;}
    let mut image:Gp=ptr::null_mut();
    if unsafe { GdipCreateBitmapFromHICON(hicon,&mut image) }==0 {
        image as isize
    } else {0}
}

pub fn release_native_icon(image:isize) {
    if image!=0 {unsafe { GdipDisposeImage(image as Gp); }}
}

pub struct SmoothCanvas { graphics: Gp }
impl SmoothCanvas {
    pub fn new(dc: isize) -> Option<Self> {
        let mut graphics: Gp = ptr::null_mut();
        if unsafe { GdipCreateFromHDC(dc, &mut graphics) } != 0 || graphics.is_null() {
            return None;
        }
        unsafe {
            GdipSetSmoothingMode(graphics, 4); // SmoothingModeAntiAlias
            GdipSetPixelOffsetMode(graphics, 4); // PixelOffsetModeHalf
            GdipSetInterpolationMode(graphics, 7); // InterpolationModeHighQualityBicubic
        }
        Some(Self { graphics })
    }
    pub fn flush(&self) { unsafe { GdipFlush(self.graphics,1); } }
    /// Render the application's real Windows icon (no artwork substitution).
    /// A larger source HICON is scaled with HQ bicubic sampling to 28 DIP.
    /// Only the alpha channel is adjusted for a muted session; original RGB
    /// pixels, including colorful PWA and Electron logos, are unchanged.
    pub fn icon(&self, cached_image:isize, x:i32, y:i32, size:i32, opacity:u8) -> bool {
        if cached_image==0 || size<=0 {return false;}
        let image=cached_image as Gp;
        let mut w=0u32;
        let mut h=0u32;
        let dimensions=unsafe { GdipGetImageWidth(image,&mut w)==0
            && GdipGetImageHeight(image,&mut h)==0 && w>0 && h>0 };
        if !dimensions { return false; }
        let mut attributes:Gp=ptr::null_mut();
        if opacity < 255 {
            let identity_alpha = [
                1.,0.,0.,0.,0.,
                0.,1.,0.,0.,0.,
                0.,0.,1.,0.,0.,
                0.,0.,0.,opacity as f32/255.,0.,
                0.,0.,0.,0.,1.,
            ];
            if unsafe { GdipCreateImageAttributes(&mut attributes) }==0 && !attributes.is_null() {
                if unsafe { GdipSetImageAttributesColorMatrix(attributes,0,1,
                    identity_alpha.as_ptr(),ptr::null(),0) }!=0 {
                    unsafe { GdipDisposeImageAttributes(attributes); }
                    attributes=ptr::null_mut();
                }
            }
        }
        let status=unsafe { GdipDrawImageRectRectI(self.graphics,image,x,y,size,size,
            0,0,w as i32,h as i32,2,attributes,ptr::null(),ptr::null()) };
        if !attributes.is_null() { unsafe { GdipDisposeImageAttributes(attributes); } }
        status==0
    }

    /// Draw an embedded white-alpha icon tinted to the active accent color.
    /// The color matrix scales R/G/B channels, preserving original alpha and
    /// antialiasing. This avoids text-font-dependent GitHub glyphs.
    pub fn tinted_icon(&self, image:isize, x:i32, y:i32, size:i32, color:u32)->bool {
        if image==0 || size<=0 {return false;}
        let red=(color & 0xff) as f32 / 255.0;
        let green=((color >> 8) & 0xff) as f32 / 255.0;
        let blue=((color >> 16) & 0xff) as f32 / 255.0;
        let scale=[
            red,0.,0.,0.,0.,
            0.,green,0.,0.,0.,
            0.,0.,blue,0.,0.,
            0.,0.,0.,1.,0.,
            0.,0.,0.,0.,1.,
        ];
        let mut attributes:Gp=ptr::null_mut();
        if unsafe {GdipCreateImageAttributes(&mut attributes)}!=0 || attributes.is_null() {
            return false;
        }
        let configured=unsafe {GdipSetImageAttributesColorMatrix(attributes,0,1,
            scale.as_ptr(),ptr::null(),0)}==0;
        let drawn=configured && unsafe {GdipDrawImageRectRectI(
            self.graphics,image as Gp,x,y,size,size,
            0,0,GITHUB_SIDE as i32,GITHUB_SIDE as i32,2,
            attributes,ptr::null(),ptr::null())}==0;
        unsafe {GdipDisposeImageAttributes(attributes);}
        drawn
    }

    /// One GDI+ path/brush for the entire live meter. Shape count is bounded
    /// and depends on the *measured* session peak, not its volume slider.
    /// Small patterned heights are decorative, NOT frequency-spectrum data.
    pub fn activity_dashes(&self, x:f32, y:f32, track_width:f32,
        peak:f32, color:u32, opacity:u8) -> bool {
        const DASH_W:f32=3.0;
        const PITCH:f32=6.0;
        if track_width<=0.0 {return false;}
        let active=(peak.clamp(0.0,1.0)*track_width).max(0.0);
        if active<DASH_W {return false;}
        let count=((active-DASH_W)/PITCH).floor() as i32+1;
        let count=count.clamp(0,64);
        if count==0 {return false;}
        let mut brush:Gp=ptr::null_mut();
        let mut path:Gp=ptr::null_mut();
        unsafe {
            if GdipCreateSolidFill((argb(color)&0x00ff_ffff)|((opacity as u32)<<24),&mut brush)!=0
                || brush.is_null() {return false;}
            if GdipCreatePath(0,&mut path)!=0 || path.is_null() {
                GdipDeleteBrush(brush);
                return false;
            }
            for i in 0..count {
                let offset=i as f32*PITCH;
                let width=DASH_W.min(active-offset).min(track_width-offset);
                if width<=0.0 {break;}
                let height=if i%7==3 || i%7==4 {3.0}else{2.0};
                GdipAddPathRectangle(path,x+offset,y+(3.0-height),width,height);
            }
            let status=GdipFillPath(self.graphics,brush,path);
            GdipDeletePath(path);
            GdipDeleteBrush(brush);
            status==0
        }
    }

    pub fn circle(&self, x: f32, y: f32, diameter: f32, color: u32) {
        let mut brush: Gp = ptr::null_mut();
        unsafe {
            if GdipCreateSolidFill(argb(color), &mut brush) == 0 {
                GdipFillEllipse(self.graphics, brush, x, y, diameter, diameter);
                GdipDeleteBrush(brush);
            }
        }
    }
    // Native GDI+ linear-gradient fill, clipped to one rounded path. This
    // avoids rebuilding dozens of tiny rectangles for every meter update.
    pub fn gradient_rounded(&self, x:f32, y:f32, w:f32, h:f32, radius:f32,
        from:u32, to:u32) {
        if w <= 0.0 || h <= 0.0 { return; }
        let start=GpPointF{x,y:y+h*0.5};
        let end=GpPointF{x:x+w.max(0.01),y:y+h*0.5};
        let mut brush:Gp=ptr::null_mut();
        unsafe {
            if GdipCreateLineBrush(&start,&end,argb(from),argb(to),0,&mut brush)!=0 {return;}
            let mut path:Gp=ptr::null_mut();
            if GdipCreatePath(0,&mut path)==0 {
                let r=radius.max(0.0).min(w*0.5).min(h*0.5);
                if r<0.5 {
                    GdipFillRectangle(self.graphics,brush,x,y,w,h);
                } else {
                    let d=r*2.0;
                    GdipAddPathArc(path,x+w-d,y,d,d,270.0,90.0);
                    GdipAddPathArc(path,x+w-d,y+h-d,d,d,0.0,90.0);
                    GdipAddPathArc(path,x,y+h-d,d,d,90.0,90.0);
                    GdipAddPathArc(path,x,y,d,d,180.0,90.0);
                    GdipClosePathFigure(path);
                    GdipFillPath(self.graphics,brush,path);
                }
                GdipDeletePath(path);
            }
            GdipDeleteBrush(brush);
        }
    }
    /// Paint a single, seamless four-stop brand gradient using a brush
    /// anchored to the full slider length. Only the filled portion is drawn,
    /// so the colors do not stretch as the user changes the volume.
    /// Returns false if GDI+ cannot create the brush/path, allowing fallback.
    pub fn gradient_rounded_fixed_stops(&self, x:f32, y:f32,
        filled_width:f32, full_width:f32, height:f32, radius:f32,
        colors:[u32;4], positions:[f32;4]) -> bool {
        if filled_width<=0.0 || full_width<=0.0 || height<=0.0
            || filled_width>full_width {return false;}
        let start=GpPointF{x,y:y+height*0.5};
        let end=GpPointF{x:x+full_width,y:y+height*0.5};
        let mut brush:Gp=ptr::null_mut();
        unsafe {
            if GdipCreateLineBrush(&start,&end,argb(colors[0]),argb(colors[3]),0,&mut brush)!=0
                || brush.is_null() {return false;}
            let stops=[argb(colors[0]),argb(colors[1]),argb(colors[2]),argb(colors[3])];
            if GdipSetLinePresetBlend(brush,stops.as_ptr(),positions.as_ptr(),4)!=0 {
                GdipDeleteBrush(brush);
                return false;
            }
            let mut path:Gp=ptr::null_mut();
            if GdipCreatePath(0,&mut path)!=0 || path.is_null() {
                GdipDeleteBrush(brush);
                return false;
            }
            let r=radius.max(0.0).min(filled_width*0.5).min(height*0.5);
            let status=if r<0.5 {
                GdipFillRectangle(self.graphics,brush,x,y,filled_width,height)
            } else {
                let d=r*2.0;
                GdipAddPathArc(path,x+filled_width-d,y,d,d,270.0,90.0);
                GdipAddPathArc(path,x+filled_width-d,y+height-d,d,d,0.0,90.0);
                GdipAddPathArc(path,x,y+height-d,d,d,90.0,90.0);
                GdipAddPathArc(path,x,y,d,d,180.0,90.0);
                GdipClosePathFigure(path);
                GdipFillPath(self.graphics,brush,path)
            };
            GdipDeletePath(path);
            GdipDeleteBrush(brush);
            status==0
        }
    }

    /// Soft, vertically-tinted panel with one anti-aliased rounded clipping
    /// path. Used behind opaque flyouts; it never reduces text opacity.
    pub fn gradient_rounded_vertical(&self, x: f32, y: f32, w: f32, h: f32, radius: f32,
        top: u32, bottom: u32) {
        if w <= 0.0 || h <= 0.0 { return; }
        let start = GpPointF { x: x + w * 0.5, y };
        let end = GpPointF { x: x + w * 0.5, y: y + h.max(0.01) };
        let mut brush: Gp = ptr::null_mut();
        unsafe {
            if GdipCreateLineBrush(&start, &end, argb(top), argb(bottom), 0, &mut brush) != 0 { return; }
            let mut path: Gp = ptr::null_mut();
            if GdipCreatePath(0, &mut path) == 0 {
                let r = radius.max(0.0).min(w * 0.5).min(h * 0.5);
                if r < 0.5 {
                    GdipFillRectangle(self.graphics, brush, x, y, w, h);
                } else {
                    let d = r * 2.0;
                    GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
                    GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
                    GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
                    GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
                    GdipClosePathFigure(path);
                    GdipFillPath(self.graphics, brush, path);
                }
                GdipDeletePath(path);
            }
            GdipDeleteBrush(brush);
        }
    }
    pub fn rounded(&self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32) {
        self.rounded_alpha(x,y,w,h,radius,color,255);
    }
    pub fn rounded_alpha(&self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32, opacity:u8) {
        if w <= 0.0 || h <= 0.0 { return; }
        let r = radius.max(0.0).min(w * 0.5).min(h * 0.5);
        let mut brush: Gp = ptr::null_mut();
        unsafe {
            if GdipCreateSolidFill((argb(color) & 0x00ff_ffff)|((opacity as u32)<<24), &mut brush) != 0 { return; }
            if r < 0.5 {
                GdipFillRectangle(self.graphics,brush,x,y,w,h);
                GdipDeleteBrush(brush);
                return;
            }
            let mut path: Gp = ptr::null_mut();
            if GdipCreatePath(0, &mut path) == 0 {
                // Round all four corners in a single anti-aliased figure.
                let d = (r * 2.0).max(0.05);
                GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
                GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
                GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
                GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
                GdipClosePathFigure(path);
                GdipFillPath(self.graphics, brush, path);
                GdipDeletePath(path);
            }
            GdipDeleteBrush(brush);
        }
    }
}
impl Drop for SmoothCanvas {
    fn drop(&mut self) { unsafe { GdipFlush(self.graphics,1); GdipDeleteGraphics(self.graphics); } }
}

#[cfg(test)]
mod about_logo_tests {
    use super::*;

    #[test]
    fn embedded_about_logo_is_premultiplied_and_transparent() {
        assert_eq!(ABOUT_BYTES.len(), ABOUT_SIDE * ABOUT_SIDE * 4);
        let pixels = about_logo_pixels();
        assert_eq!(&pixels[0..4], &[0, 0, 0, 0]);
        assert!(pixels.chunks_exact(4).any(|p| p[3] == 255));
        assert!(pixels.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
        for px in pixels.chunks_exact(4) {
            assert!(px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]);
        }
    }
}
