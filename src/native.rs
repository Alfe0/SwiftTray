//! Minimal Win32 FFI surface: no GUI runtime, no WebView2, no Electron.
//! All HWND/HDC/HICON/HBRUSH/HFONT handles are pointer-sized integers.
use std::ffi::c_void;

pub type H = isize;
pub type Proc = unsafe extern "system" fn(H, u32, usize, isize) -> isize;
#[repr(C)] #[derive(Clone, Copy, Default, Debug)]
pub struct Rect { pub left:i32, pub top:i32, pub right:i32, pub bottom:i32 }
impl Rect {
    pub fn new(x:i32,y:i32,w:i32,h:i32) -> Self { Self {left:x,top:y,right:x+w,bottom:y+h} }
    pub fn contains(&self,x:i32,y:i32)->bool { x>=self.left&&x<self.right&&y>=self.top&&y<self.bottom }
    pub fn width(&self)->i32 { self.right-self.left }
    pub fn height(&self)->i32 { self.bottom-self.top }
}
#[repr(C)] #[derive(Clone, Copy, Default)]
pub struct Point { pub x:i32, pub y:i32 }
#[repr(C)]
pub struct TrackMouseEventData {
    pub size:u32, pub flags:u32, pub window:H, pub hover_time:u32,
}
#[repr(C)]
pub struct Msg { pub hwnd:H, pub message:u32, pub wparam:usize, pub lparam:isize, pub time:u32, pub pt:Point, pub private:u32 }
#[repr(C)]
pub struct WindowClass {
    pub size:u32, pub style:u32, pub callback:Option<Proc>, pub extra_class:i32,
    pub extra_window:i32, pub instance:H, pub icon:H, pub cursor:H,
    pub background:H, pub menu_name:*const u16, pub class_name:*const u16, pub small_icon:H,
}
#[repr(C)]
pub struct PaintStruct {
    pub hdc:H, pub erase:i32, pub paint:Rect, pub restore:i32,
    pub update:i32, pub reserved:[u8;32],
}
#[repr(C)] #[derive(Clone, Copy)]
pub struct MonitorInfo { pub size:u32, pub monitor:Rect, pub work:Rect, pub flags:u32 }
#[repr(C)] #[derive(Clone, Copy)]
pub struct Margins { pub left:i32, pub right:i32, pub top:i32, pub bottom:i32 }
#[repr(C)]
pub struct NotifyIconData {
    pub size:u32, pub window:H, pub id:u32, pub flags:u32, pub callback:u32,
    pub icon:H, pub tip:[u16;128], pub state:u32, pub state_mask:u32,
    pub info:[u16;256], pub timeout_or_version:u32, pub title:[u16;64],
    pub info_flags:u32, pub guid:[u8;16], pub balloon_icon:H,
}

pub const WM_DESTROY:u32=2;
pub const WM_PAINT:u32=0x000F; pub const WM_CLOSE:u32=0x0010; pub const WM_CTLCOLOREDIT:u32=0x0133;
pub const WM_COMMAND:u32=0x0111; pub const WM_TIMER:u32=0x0113;
pub const WM_KEYDOWN:u32=0x0100; pub const WM_SYSKEYDOWN:u32=0x0104;
pub const WM_KEYUP:u32=0x0101; pub const WM_SYSKEYUP:u32=0x0105;
pub const WH_KEYBOARD_LL:i32=13;
#[repr(C)]
pub struct KeyboardLowLevelData {
    pub vk_code:u32, pub scan_code:u32, pub flags:u32,
    pub time:u32, pub extra_info:usize,
}
pub type HookCallback = unsafe extern "system" fn(i32, usize, isize) -> isize;
pub type WinEventCallback = unsafe extern "system" fn(H,u32,H,i32,i32,u32,u32);
pub const WM_LBUTTONDOWN:u32=0x0201; pub const WM_LBUTTONUP:u32=0x0202;
pub const WM_MOUSEMOVE:u32=0x0200;
pub const WM_MOUSELEAVE:u32=0x02A3; pub const TME_LEAVE:u32=0x00000002;
pub const WM_MOUSEWHEEL:u32=0x020A;
pub const WM_CAPTURECHANGED:u32=0x0215;
pub const WM_SETTINGCHANGE:u32=0x001A;
pub const WM_SYSCOLORCHANGE:u32=0x0015;
pub const WM_THEMECHANGED:u32=0x031A;
pub const WM_DEVICECHANGE:u32=0x0219;
pub const WM_DISPLAYCHANGE:u32=0x007E;
pub const WM_DPICHANGED:u32=0x02E0;
pub const WM_DWMCOLORIZATIONCOLORCHANGED:u32=0x0320;
pub const WM_ACTIVATE:u32=0x0006; pub const WM_HOTKEY:u32=0x0312;
pub const WM_CLIPBOARDUPDATE:u32=0x031D;
pub const WM_APP:u32=0x8000;
pub const SPI_GETHIGHCONTRAST:u32=0x0042;
pub const HCF_HIGHCONTRASTON:u32=0x00000001;
#[repr(C)]
pub struct HighContrast {
    pub size:u32, pub flags:u32, pub default_scheme:*mut u16,
}
pub const WS_POPUP:u32=0x80000000; pub const WS_CHILD:u32=0x40000000;
pub const WS_BORDER:u32=0x00800000;
pub const WS_TABSTOP:u32=0x00010000;
pub const WS_EX_TOPMOST:u32=0x00000008; pub const WS_EX_TOOLWINDOW:u32=0x00000080;
pub const WS_EX_NOACTIVATE:u32=0x08000000;
pub const WS_EX_LAYERED:u32=0x00080000;
pub const GWL_EXSTYLE:i32=-20;
pub const LWA_ALPHA:u32=0x00000002;
pub const ES_AUTOHSCROLL:u32=0x0080;
pub const SW_HIDE:i32=0; pub const SW_SHOWNORMAL:i32=1;
pub const SWP_NOACTIVATE:u32=0x0010; pub const SWP_SHOWWINDOW:u32=0x0040;
pub const SWP_NOZORDER:u32=0x0004;
pub const DT_SINGLELINE:u32=0x0020; pub const DT_VCENTER:u32=0x0004;
pub const DT_CALCRECT:u32=0x00000400;
pub const DT_END_ELLIPSIS:u32=0x00008000; pub const DT_NOPREFIX:u32=0x00000800;
pub const DT_CENTER:u32=0x00000001; pub const DT_RIGHT:u32=0x00000002;
pub const TRANSPARENT:i32=1; pub const PS_SOLID:i32=0;
pub const SRCCOPY:u32=0x00CC0020;
pub const BLACKNESS:u32=0x00000042;
pub const NIF_MESSAGE:u32=1; pub const NIF_ICON:u32=2; pub const NIF_TIP:u32=4;
// Required to show the standard Windows tooltip when using NOTIFYICON_VERSION_4.
pub const NIF_SHOWTIP:u32=0x00000080;
pub const NIM_ADD:u32=0; pub const NIM_DELETE:u32=2; pub const NIM_SETVERSION:u32=4;
pub const NOTIFYICON_VERSION_4:u32=4;
pub const TPM_LEFTALIGN:u32=0; pub const TPM_BOTTOMALIGN:u32=0x20; pub const TPM_RETURNCMD:u32=0x0100;
pub const MF_STRING:u32=0; pub const MOD_NOREPEAT:u32=0x4000;
pub const VK_CONTROL:i32=0x11; pub const VK_MENU:i32=0x12;
pub const VK_SHIFT:i32=0x10; pub const VK_LWIN:i32=0x5B; pub const VK_RWIN:i32=0x5C;
pub const VK_ESCAPE:u32=0x1B;
pub const EN_KILLFOCUS:u32=0x0200;
pub const WM_SETFONT:u32=0x0030;
pub const INPUT_KEYBOARD:u32=1;
pub const KEYEVENTF_KEYUP:u32=0x0002;

#[repr(C)] #[derive(Clone, Copy)]
pub struct KeyboardInput {
    pub virtual_key:u16, pub scan:u16, pub flags:u32, pub time:u32, pub extra_info:usize,
}
#[repr(C)] #[derive(Clone, Copy)]
pub struct MouseInput {
    pub dx:i32, pub dy:i32, pub mouse_data:u32, pub flags:u32, pub time:u32, pub extra_info:usize,
}
#[repr(C)] #[derive(Clone, Copy)]
pub struct HardwareInput { pub message:u32, pub low:u16, pub high:u16 }
#[repr(C)] #[derive(Clone, Copy)]
pub union WinInputData { pub keyboard:KeyboardInput, pub mouse:MouseInput, pub hardware:HardwareInput }
#[repr(C)] #[derive(Clone, Copy)]
pub struct WinInput { pub kind:u32, pub data:WinInputData }


pub fn wide(text:&str)->Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
pub fn rgb(r:u8,g:u8,b:u8)->u32 { r as u32 | (g as u32)<<8 | (b as u32)<<16 }
pub fn scaled(v:i32,dpi:u32)->i32 { (v as i64 * dpi as i64 / 96) as i32 }
pub fn copy_wide<const N:usize>(target:&mut [u16;N], text:&str) {
    for (i,c) in text.encode_utf16().take(N-1).enumerate() {target[i]=c;}
}
pub fn x_of(lp:isize)->i32 { (lp as u16) as i16 as i32 }
pub fn y_of(lp:isize)->i32 { ((lp>>16) as u16) as i16 as i32 }

#[link(name="user32")]
extern "system" {
    pub fn SystemParametersInfoW(action:u32,parameter:u32,value:*mut c_void,flags:u32)->i32;
    pub fn GetSysColor(index:i32)->u32;
    pub fn RegisterClassExW(class:*const WindowClass)->u16;
    pub fn GetWindowLongPtrW(hwnd:H,index:i32)->isize;
    pub fn SetWindowLongPtrW(hwnd:H,index:i32,value:isize)->isize;
    pub fn SetLayeredWindowAttributes(hwnd:H,color_key:u32,alpha:u8,flags:u32)->i32;
    pub fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,
        x:i32,y:i32,width:i32,height:i32,parent:H,menu:H,instance:H,param:*mut c_void)->H;
    pub fn DefWindowProcW(hwnd:H,msg:u32,w:usize,l:isize)->isize;
    pub fn DestroyWindow(hwnd:H)->i32;
    pub fn ShowWindow(hwnd:H,command:i32)->i32;
    pub fn SetForegroundWindow(hwnd:H)->i32;
    pub fn SetFocus(hwnd:H)->H;
    pub fn GetMessageW(msg:*mut Msg,window:H,min:u32,max:u32)->i32;
    pub fn TranslateMessage(msg:*const Msg)->i32;
    pub fn DispatchMessageW(msg:*const Msg)->isize;
    pub fn PostQuitMessage(exit:i32);
    pub fn PostMessageW(hwnd:H,msg:u32,w:usize,l:isize)->i32;
    pub fn SetWindowsHookExW(id_hook:i32,hook:Option<HookCallback>,module:H,thread_id:u32)->H;
    pub fn UnhookWindowsHookEx(hook:H)->i32;
    pub fn SetWinEventHook(min:u32,max:u32,module:H,callback:Option<WinEventCallback>,
        process:u32,thread:u32,flags:u32)->H;
    pub fn UnhookWinEvent(hook:H)->i32;
    pub fn GetForegroundWindow()->H;
    pub fn GetWindowThreadProcessId(hwnd:H,pid:*mut u32)->u32;
    pub fn AddClipboardFormatListener(hwnd:H)->i32;
    pub fn RemoveClipboardFormatListener(hwnd:H)->i32;
    pub fn CallNextHookEx(hook:H,code:i32,w:usize,l:isize)->isize;
    pub fn InvalidateRect(hwnd:H,rect:*const Rect,erase:i32)->i32;
    pub fn BeginPaint(hwnd:H,paint:*mut PaintStruct)->H;
    pub fn EndPaint(hwnd:H,paint:*const PaintStruct)->i32;
    pub fn GetClientRect(hwnd:H,rect:*mut Rect)->i32;
    pub fn SetWindowPos(hwnd:H,after:H,x:i32,y:i32,w:i32,h:i32,flags:u32)->i32;
    pub fn MoveWindow(hwnd:H,x:i32,y:i32,w:i32,h:i32,repaint:i32)->i32;
    pub fn RegisterHotKey(hwnd:H,id:i32,modifiers:u32,key:u32)->i32;
    pub fn UnregisterHotKey(hwnd:H,id:i32)->i32;
    pub fn GetKeyState(key:i32)->i16;
    pub fn GetAsyncKeyState(key:i32)->i16;
    pub fn SendInput(count:u32, inputs:*mut WinInput, size:i32)->u32;
    pub fn RegisterWindowMessageW(name:*const u16)->u32;
    pub fn SetTimer(hwnd:H,id:usize,delay:u32,callback:usize)->usize;
    pub fn KillTimer(hwnd:H,id:usize)->i32;
    pub fn SetCapture(hwnd:H)->H;
    pub fn ReleaseCapture()->i32;
    pub fn GetDpiForWindow(hwnd:H)->u32;
    pub fn GetDpiForSystem()->u32;
    pub fn SetProcessDpiAwarenessContext(context:isize)->i32;
    pub fn MonitorFromWindow(hwnd:H,flags:u32)->H;
    pub fn GetMonitorInfoW(monitor:H,info:*mut MonitorInfo)->i32;
    pub fn GetCursorPos(point:*mut Point)->i32;
    pub fn TrackMouseEvent(event:*mut TrackMouseEventData)->i32;
    pub fn ScreenToClient(hwnd:H,point:*mut Point)->i32;
    pub fn LoadIconW(instance:H,name:*const u16)->H;
    pub fn LoadImageW(instance:H,name:*const u16,image_type:u32,cx:i32,cy:i32,load:u32)->H;
    pub fn DestroyIcon(icon:H)->i32;
    pub fn LoadCursorW(instance:H,name:*const u16)->H;
    pub fn DrawIconEx(hdc:H,x:i32,y:i32,icon:H,w:i32,h:i32,step:u32,brush:H,flags:u32)->i32;
    pub fn DrawStateW(hdc:H,brush:H,callback:usize,data:isize,wdata:usize,x:i32,y:i32,cx:i32,cy:i32,flags:u32)->i32;
    pub fn DrawTextW(hdc:H,text:*const u16,count:i32,rect:*mut Rect,flags:u32)->i32;
    pub fn SetWindowTextW(hwnd:H,text:*const u16)->i32;
    pub fn GetWindowTextW(hwnd:H,text:*mut u16,limit:i32)->i32;
    pub fn SendMessageW(hwnd:H,msg:u32,w:usize,l:isize)->isize;
    pub fn CreatePopupMenu()->H;
    pub fn AppendMenuW(menu:H,flags:u32,id:usize,text:*const u16)->i32;
    pub fn TrackPopupMenu(menu:H,flags:u32,x:i32,y:i32,reserved:i32,owner:H,rect:*const Rect)->u32;
    pub fn DestroyMenu(menu:H)->i32;
    pub fn MessageBoxW(hwnd:H,text:*const u16,title:*const u16,flags:u32)->i32;
}
#[link(name="gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc:H)->H;
    pub fn CreateCompatibleBitmap(hdc:H,width:i32,height:i32)->H;
    pub fn SelectObject(hdc:H,obj:H)->H;
    pub fn DeleteObject(obj:H)->i32;
    pub fn DeleteDC(hdc:H)->i32;
    pub fn BitBlt(dest:H,x:i32,y:i32,w:i32,h:i32,src:H,sx:i32,sy:i32,rop:u32)->i32;
    pub fn PatBlt(hdc:H,x:i32,y:i32,width:i32,height:i32,rop:u32)->i32;
    pub fn CreateSolidBrush(color:u32)->H;
    pub fn CreatePen(style:i32,width:i32,color:u32)->H;
    pub fn RoundRect(hdc:H,left:i32,top:i32,right:i32,bottom:i32,ww:i32,hh:i32)->i32;
    pub fn Rectangle(hdc:H,left:i32,top:i32,right:i32,bottom:i32)->i32;
    pub fn Ellipse(hdc:H,left:i32,top:i32,right:i32,bottom:i32)->i32;
    pub fn SetBkMode(hdc:H,mode:i32)->i32;
    pub fn SetTextColor(hdc:H,color:u32)->u32;
    pub fn SetBkColor(hdc:H,color:u32)->u32;
    pub fn CreateFontW(height:i32,width:i32,esc:i32,orient:i32,weight:i32,
       italic:u32,underline:u32,strike:u32,charset:u32,outprec:u32,clipprec:u32,
       quality:u32,pitch:u32,name:*const u16)->H;
}
#[link(name="kernel32")]
extern "system" { pub fn GetModuleHandleW(name:*const u16)->H;
    pub fn GetTickCount()->u32; }
#[link(name="shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message:u32,data:*mut NotifyIconData)->i32;
    pub fn ShellExecuteW(hwnd:H,operation:*const u16,file:*const u16,params:*const u16,dir:*const u16,show:i32)->H;
}
#[link(name="runtimeobject")]
extern "system" {
    pub fn RoInitialize(init_type:i32)->i32;
    pub fn RoUninitialize();
}
#[link(name="dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd:H,attr:u32,value:*const c_void,size:u32)->i32;
    pub fn DwmExtendFrameIntoClientArea(hwnd:H,margins:*const Margins)->i32;
    pub fn DwmGetColorizationColor(color:*mut u32,opaque:*mut i32)->i32;
}
