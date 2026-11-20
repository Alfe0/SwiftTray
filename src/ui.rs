use crate::{audio::Audio, capture::{self, MSG_CAPTURE_KEYS_RELEASED}, config::{Config, Shortcut},
    flyout_lifecycle::{CaptureSession, MSG_CAPTURE_FOREGROUND},
    hotkeys::{self, HotkeyManager, HOTKEY_POPUP, HOTKEY_SWITCH, HOTKEY_SNIP, MSG_F12_SHORTCUT, VK_F12},
    backdrop, brand::{self, BRAND_GREEN, BRAND_TEAL, BRAND_BLUE, BRAND_DEEP_BLUE, BRAND_HOVER, BRAND_METER, BRAND_INK},
    icons::IconCache, native::*, smooth, startup, updates};
use std::{ffi::c_void, mem, ptr, sync::mpsc::{self, Receiver}};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::UI::ViewManagement::{UIColorType, UISettings};

const WIDTH: i32 = 360;
const ROW_HEIGHT: i32 = 46;
const MIXER_FIRST: i32 = 94;
const SLIDER_LEFT: i32 = 62;
const SLIDER_RIGHT: i32 = WIDTH - 25;
const TRAY_EVENT: u32 = WM_APP + 1;
const TRAY_TOOLTIP: &str = "SwiftTray";
const TIMER_REFRESH: usize = 1;
const TIMER_TOAST: usize = 2;
const TIMER_SLIDER: usize = 3;
const TIMER_APP_TOOLTIP: usize = 5;
const TIMER_METER: usize = 6;
const TIMER_HOTKEY_RETRY: usize = 7;
const TIMER_UPDATE_RESULT: usize = 8;
const SYSTEM_ICON: *const u16 = 32512usize as *const u16;
const ARROW_CURSOR: *const u16 = 32512usize as *const u16;
const EDIT_ID: usize = 300;
// Layout reference points (DIPs) for the compact About and tray menu surfaces.
const ABOUT_LOGO_TOP: i32 = 64;
const ABOUT_LOGO_SIZE: i32 = 92;
const ABOUT_FOOTER_RULE: i32 = 342;
const TRAY_EXIT_OFFSET: i32 = 11;

static mut APP: *mut App = ptr::null_mut();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page { Mixer, Settings, Appearance, Devices, DeviceIcons, Shortcuts, Notifications, About, Welcome, TrayMenu }
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Settings, Back, Switch,
    Page(Page), DeviceToggle(usize), DeviceRename(usize), DeviceIcon(usize), PickIcon(usize),
    SaveAlias, Capture(usize), ToastToggle, SetToastPosition(u8),
    StartupToggle, StartupEnable, StartupSkip, TrayAbout, TrayCheckUpdates, SetAccentMode(bool),
    CheckUpdates, OpenUpdateRelease, OpenRepository, OpenLicense,
    MasterMute, AppMute(usize),
    MasterSlider, AppSlider(usize), NextApps, TrayOpen, TraySettings, TrayExit,
}
#[derive(Clone)]
struct Hit { rect: Rect, action: Action }

enum UpdateStatus {
    Idle, Checking, Unconfigured, Current(String),
    Available { version:String, url:String }, Error(String),
}

struct App {
    window:H, toast:H, edit:H, edit_font:H, edit_brush:H,
    dpi:u32, config:Config, audio:Option<Audio>, icons:IconCache,
    page:Page, hits:Vec<Hit>, visible:bool,
    palette:Palette,
    dragging:Option<Action>, pending_slider:Option<(Action,f32)>,
    last_audio_send:u32, slider_timer_armed:bool, hover:Option<Action>,
    tooltip_visible:bool, tracking_mouse:bool, capture_shortcut:Option<usize>,
    edit_device:Option<usize>, icon_device:Option<usize>, page_offset:usize, mixer_rows_floor:usize,
    toast_text:String, toast_symbol:String, capture_session:CaptureSession,
    status:String, hotkey_status:[bool;3], shortcut_errors:[Option<String>;3], hotkey_manager:HotkeyManager,
    tray_icon:H, tray_menu_icon:H, about_icon:H, tray_owned:bool,
    tray_image:isize, about_image:isize, about_pixels:Vec<u8>, taskbar_created:u32, tray_choice:usize,
    startup_on:bool, startup_error:String,
    update_status:UpdateStatus, update_receiver:Option<Receiver<Result<updates::ResultState,String>>>,
}
impl App {
    fn new(config:Config, audio:Option<Audio>) -> Self {
        let palette=Palette::from_config(&config);
        Self { window:0, toast:0, edit:0, edit_font:0, edit_brush:0, dpi:96,
            config, audio, icons:IconCache::new(), page:Page::Mixer,
            hits:Vec::new(), visible:false, palette,
            dragging:None, pending_slider:None, last_audio_send:0, slider_timer_armed:false, hover:None,
            tooltip_visible:false, tracking_mouse:false,
            capture_shortcut:None, edit_device:None, icon_device:None, page_offset:0, mixer_rows_floor:0,
            toast_text:String::new(), toast_symbol:"\u{E767}".into(), capture_session:CaptureSession::new(),
            status:String::new(), hotkey_status:[false;3], shortcut_errors:[None,None,None], hotkey_manager:HotkeyManager::new(),
            tray_icon:0, tray_menu_icon:0, about_icon:0, tray_owned:false,
            tray_image:0, about_image:0, about_pixels:Vec::new(), taskbar_created:0, tray_choice:0,
            startup_on:startup::is_enabled(), startup_error:String::new(),
            update_status:UpdateStatus::Idle, update_receiver:None }
    }
    fn s(&self,v:i32)->i32 { scaled(v,self.dpi) }
    fn workarea(&self)->Rect {
        unsafe {
            let mon=MonitorFromWindow(self.window,2);
            let mut info=MonitorInfo {size:mem::size_of::<MonitorInfo>() as u32, monitor:Rect::default(),work:Rect::default(),flags:0};
            if mon!=0 && GetMonitorInfoW(mon,&mut info)!=0 { return info.work; }
        }
        Rect::new(0,0,1920,1080)
    }
    fn page_width(&self)->i32 { if self.page==Page::TrayMenu { 230 } else { WIDTH } }
    fn page_size(&self)->usize {
        // No scrollbars. Limit rows to available physical work area and page the rest.
        let h=self.workarea().height()*96/self.dpi as i32;
        let fixed=MIXER_FIRST;
        ((h - fixed - 44) / ROW_HEIGHT - 1).clamp(1,10) as usize
    }
    fn page_height(&self)->i32 {
        match self.page {
            Page::Mixer => {
                let apps=self.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                let rows=apps.max(self.mixer_rows_floor).saturating_sub(self.page_offset).min(self.page_size());
                MIXER_FIRST + ROW_HEIGHT * (rows as i32 + 1)
                    + if apps>self.page_size() { 38 } else { 13 }
            },
            Page::Settings => 479,
            Page::Appearance => 333,
            Page::Devices => 150+58*(self.audio.as_ref().map(|a|a.devices.len().min(6)).unwrap_or(0) as i32),
            Page::DeviceIcons => 306,
            Page::Shortcuts => 329,
            Page::Notifications => 304,
            Page::About => 394,
            Page::Welcome => 244,
            Page::TrayMenu => 265,
        }
    }
    fn resize(&mut self) {
        let work=self.workarea();
        let w=self.s(self.page_width()); let h=self.s(self.page_height());
        let x=work.right-w-self.s(14); let y=work.bottom-h-self.s(14);
        unsafe { SetWindowPos(self.window,-1,x,y,w,h,SWP_NOACTIVATE); }
    }
    fn show_tray_menu(&mut self) {
        self.page=Page::TrayMenu;
        self.tray_choice=0;
        self.edit_device=None;
        self.hover=None; self.tooltip_visible=false;
        unsafe {
            KillTimer(self.window,TIMER_APP_TOOLTIP);
            KillTimer(self.window,TIMER_METER);
            KillTimer(self.window,TIMER_REFRESH);
            ShowWindow(self.edit,SW_HIDE);
        }
        self.palette=Palette::from_config(&self.config);
        self.dpi=unsafe {GetDpiForWindow(self.window).max(96)};
        let mut cursor=Point::default();
        unsafe {GetCursorPos(&mut cursor);}
        let work=self.workarea();
        let w=self.s(self.page_width());let h=self.s(self.page_height());
        let x=cursor.x.min(work.right-w-self.s(6)).max(work.left+self.s(6));
        let y=(cursor.y-h-self.s(8)).max(work.top+self.s(6));
        self.visible=true;
        unsafe {
            SetWindowPos(self.window,-1,x,y,w,h,SWP_NOACTIVATE|SWP_SHOWWINDOW);
            SetForegroundWindow(self.window);
            InvalidateRect(self.window,ptr::null(),0);
        }
    }
    fn show_welcome(&mut self) {
        self.page=Page::Welcome;
        self.palette=Palette::from_config(&self.config);
        self.dpi=unsafe {GetDpiForWindow(self.window).max(96)};
        self.resize();
        self.visible=true;
        unsafe {
            ShowWindow(self.window,SW_SHOWNORMAL);
            SetForegroundWindow(self.window);
            InvalidateRect(self.window,ptr::null(),0);
        }
    }
    fn finish_welcome(&mut self, enable:bool) {
        self.config.startup_prompt_seen=true;
        let _=self.config.save();
        if enable {
            match startup::set_enabled(true) {
                Ok(())=>{self.startup_on=true;self.startup_error.clear();},
                Err(e)=>{self.startup_on=false;self.startup_error=e;},
            }
        }
        self.show();
        if !self.startup_error.is_empty() {self.go(Page::Settings);}
    }
    fn toggle_startup(&mut self) {
        let target=!startup::is_enabled();
        match startup::set_enabled(target) {
            Ok(())=>{
                self.startup_on=startup::is_enabled();
                self.startup_error.clear();
                if target && !self.startup_on {
                    self.startup_error="Windows did not retain the startup setting".into();
                }
            },
            Err(error)=>self.startup_error=error,
        }
        self.repaint();
    }
    fn toggle(&mut self) {
        if self.visible { self.hide(); } else { self.show(); }
    }
    fn show(&mut self) {
        self.page=Page::Mixer; self.page_offset=0; self.tooltip_visible=false;
        unsafe { KillTimer(self.window,TIMER_APP_TOOLTIP); }
        self.palette=Palette::from_config(&self.config);
        if let Some(a)=self.audio.as_mut() { let _=a.refresh(); }
        else { self.audio=Audio::new().ok(); }
        self.mixer_rows_floor=self.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
        self.dpi=unsafe { GetDpiForWindow(self.window).max(96) };
        self.resize(); self.visible=true;
        unsafe {
            ShowWindow(self.window,SW_SHOWNORMAL);
            SetForegroundWindow(self.window);
            SetTimer(self.window,TIMER_REFRESH,2200,0);
            SetTimer(self.window,TIMER_METER,40,0);
            InvalidateRect(self.window,ptr::null(),0);
        }
    }
    fn hide(&mut self) {
        self.capture_session.finish(self.window);
        if self.page==Page::Welcome && !self.config.startup_prompt_seen {
            self.config.startup_prompt_seen=true;
            let _=self.config.save();
        }
        self.cancel_capture(); self.dragging=None; self.pending_slider=None; self.hover=None;
        self.tooltip_visible=false; self.tracking_mouse=false;
        unsafe { KillTimer(self.window,TIMER_APP_TOOLTIP); }
        unsafe { KillTimer(self.window,TIMER_SLIDER); }
        self.slider_timer_armed=false;
        self.edit_device=None;
        unsafe {
            KillTimer(self.window,TIMER_REFRESH);
            KillTimer(self.window,TIMER_METER);
            ShowWindow(self.edit,SW_HIDE);
            ShowWindow(self.window,SW_HIDE);
        }
        self.visible=false;
    }
    fn go(&mut self, page:Page) {
        self.cancel_capture();
        self.page=page; self.edit_device=None; self.hover=None;
        unsafe {
            if page==Page::Mixer && self.visible {
                SetTimer(self.window,TIMER_METER,40,0);
            } else { KillTimer(self.window,TIMER_METER); }
        }
        if page==Page::Settings {self.startup_on=startup::is_enabled();}
        self.tooltip_visible=false;
        unsafe { KillTimer(self.window,TIMER_APP_TOOLTIP); }
        unsafe { ShowWindow(self.edit,SW_HIDE); }
        self.resize(); self.repaint();
    }
    fn hover_is_audio_icon(&self) -> bool {
        self.page==Page::Mixer && matches!(self.hover,Some(Action::MasterMute|Action::AppMute(_)))
    }
    fn update_hover(&mut self,next:Option<Action>) {
        if next==self.hover { return; }
        self.hover=next;
        self.tooltip_visible=false;
        unsafe { KillTimer(self.window,TIMER_APP_TOOLTIP); }
        if self.hover_is_audio_icon() {
            unsafe { SetTimer(self.window,TIMER_APP_TOOLTIP,420,0); }
        }
        if let Some(action)=next {
            self.tray_choice=match action {
                Action::TrayOpen=>0,Action::TraySettings=>1,Action::TrayCheckUpdates=>2,Action::TrayAbout=>3,Action::TrayExit=>4,
                _=>self.tray_choice,
            };
        }
        self.repaint();
    }
    fn repaint(&self) { unsafe { InvalidateRect(self.window,ptr::null(),0); } }
    fn binding(&self, index:usize) -> Shortcut {
        match index {0=>self.config.popup,1=>self.config.switch,_=>self.config.snip}
    }
    fn unregister_binding(&mut self,index:usize) {
        self.hotkey_manager.unregister(self.window,index,self.binding(index));
    }
    fn register_binding(&mut self,index:usize,shortcut:Shortcut)->bool {
        self.hotkey_manager.register(self.window,index,shortcut)
    }
    fn schedule_hotkey_retry(&self) {
        unsafe {
            if self.hotkey_status.iter().all(|registered| *registered) {
                KillTimer(self.window,TIMER_HOTKEY_RETRY);
            } else {
                // Retry only missing registrations; never steal an existing hotkey.
                SetTimer(self.window,TIMER_HOTKEY_RETRY,10_000,0);
            }
        }
    }
    fn retry_unavailable_hotkeys(&mut self) {
        if self.capture_shortcut.is_some() { return; }
        let mut changed=false;
        for index in 0..3 {
            if !self.hotkey_status[index] && self.register_binding(index,self.binding(index)) {
                self.hotkey_status[index]=true;
                self.shortcut_errors[index]=None;
                changed=true;
            }
        }
        self.schedule_hotkey_retry();
        if changed {self.repaint();}
    }
    fn hotkeys(&mut self) {
        for index in 0..3 {
            self.unregister_binding(index);
            let key=self.binding(index);
            self.hotkey_status[index]=self.register_binding(index,key);
        }
        if !self.hotkey_status[0] && self.config.popup==Shortcut::new(0x000A,b'V' as u32) {
            let fallback=Shortcut::new(0x0003,b'V' as u32);
            if self.register_binding(0,fallback) {
                self.config.popup=fallback;
                self.hotkey_status[0]=true;
                let _=self.config.save();
            }
        }
        self.schedule_hotkey_retry();
    }
    fn start_capture(&mut self,index:usize) {
        self.cancel_capture();
        unsafe {KillTimer(self.window,TIMER_HOTKEY_RETRY);}
        // Suspend all three bindings so a captured key does not trigger another
        // SwiftTray action before we can display a duplicate-binding error.
        for i in 0..3 {
            self.unregister_binding(i);
            self.hotkey_status[i]=false;
        }
        self.shortcut_errors[index]=None;
        self.capture_shortcut=Some(index);
        self.status.clear();
        self.repaint();
    }
    fn restore_bindings(&mut self,except:Option<usize>) {
        for i in 0..3 {
            if except==Some(i) {continue;}
            let key=self.binding(i);
            self.hotkey_status[i]=self.register_binding(i,key);
        }
        self.schedule_hotkey_retry();
    }
    fn cancel_capture(&mut self) {
        if self.capture_shortcut.take().is_some() {
            self.restore_bindings(None);
        }
    }
    fn assign_hotkey(&mut self,index:usize,new:Shortcut) {
        if [self.config.popup,self.config.switch,self.config.snip]
            .iter().enumerate().any(|(i,k)| i!=index && *k==new) {
            self.shortcut_errors[index]=Some("Already assigned to another SwiftTray action".into());
            self.restore_bindings(None);
            return;
        }
        if self.register_binding(index,new) {
            match index {0=>self.config.popup=new,1=>self.config.switch=new,_=>self.config.snip=new}
            self.hotkey_status[index]=true;
            self.shortcut_errors[index]=None;
            if new.modifiers==0 && ((0x30..=0x39).contains(&new.key) || (0x41..=0x5A).contains(&new.key)) {
                self.shortcut_errors[index]=Some("A single letter also triggers while typing".into());
            }
            let _=self.config.save();
            self.restore_bindings(Some(index));
        } else {
            self.shortcut_errors[index]=Some(if new.key==VK_F12 {
                "F12 is unavailable on this Windows session".into()
            } else {
                "Reserved by Windows or another application".into()
            });
            self.restore_bindings(None);
        }
    }
    fn begin_update_check(&mut self) {
        if matches!(&self.update_status,UpdateStatus::Checking) {return;}
        // Local builds have no repository until the owner configures one.
        // Report this as a neutral setup state, not a fatal red error.
        if !updates::is_configured() {
            self.update_status=UpdateStatus::Unconfigured;
            self.repaint();
            return;
        }
        self.update_status=UpdateStatus::Checking;
        let (tx,rx)=mpsc::channel();
        self.update_receiver=Some(rx);
        std::thread::spawn(move || {
            let result=updates::check_latest();
            let _=tx.send(result);
        });
        unsafe {SetTimer(self.window,TIMER_UPDATE_RESULT,150,0);}
        self.repaint();
    }
    fn receive_update_result(&mut self) {
        let result=self.update_receiver.as_ref().map(|rx|rx.try_recv());
        let state=match result {
            Some(Ok(Ok(updates::ResultState::Current{version})))=>Some(UpdateStatus::Current(version)),
            Some(Ok(Ok(updates::ResultState::Available{version,url})))=>Some(UpdateStatus::Available{version,url}),
            Some(Ok(Err(error)))=>Some(UpdateStatus::Error(error)),
            Some(Err(mpsc::TryRecvError::Disconnected))=>
                Some(UpdateStatus::Error("Update check ended unexpectedly".into())),
            _=>None,
        };
        if let Some(state)=state {
            unsafe {KillTimer(self.window,TIMER_UPDATE_RESULT);}
            self.update_receiver=None;
            self.update_status=state;
            self.repaint();
        }
    }
    fn open_url(&mut self,url:&str) {
        let file=wide(url);
        let operation=wide("open");
        let code=unsafe {ShellExecuteW(self.window,operation.as_ptr(),file.as_ptr(),
            ptr::null(),ptr::null(),SW_SHOWNORMAL)} as isize;
        if code<=32 {self.status="Could not open the link in your browser".into();}
    }
    fn next_device(&mut self) {
        match self.audio.as_mut() {
            Some(audio)=>match audio.switch_to_next(&self.config) {
                Ok(Some(d))=>{
                    self.toast_text=self.config.display_name(&d.id,&d.name).to_owned();
                    self.toast_symbol=device_glyph(&self.config,&d.id,&d.name).into();
                    self.mixer_rows_floor=self.mixer_rows_floor.max(audio.sessions.len());
                    self.page_offset=0;
                    self.resize();self.repaint();
                    if self.config.toast_enabled { self.show_toast(); }
                }
                Ok(None)=>{
                    self.status="Enable at least two audio devices in Settings".into();
                    self.toast_text=self.status.clone();self.toast_symbol="\u{E7F4}".into();
                    if self.config.toast_enabled { self.show_toast(); }
                    self.repaint();
                }
                Err(err)=>{
                    self.status=err;
                    self.toast_text="Could not switch audio device".into();
                    self.toast_symbol="!".into();
                    if self.config.toast_enabled {self.show_toast();}
                    self.repaint();
                }
            },
            None=> {self.status="Windows audio not available".into();self.repaint();}
        }
    }
    fn show_toast(&self) {
        let work=self.workarea();
        let w=self.s(248);let h=self.s(62);
        let (x,y)=match self.config.popup_position() {
            1=>(work.left+self.s(16),work.top+self.s(18)),
            2=>{
                let y=(work.bottom-self.s(self.page_height())-self.s(14)-h-self.s(10))
                    .max(work.top+self.s(10));
                (work.right-w-self.s(14),y)
            },
            _=>(work.right-w-self.s(16),work.top+self.s(18)),
        };
        unsafe {
            SetWindowPos(self.toast,-1,x,y,w,h,SWP_NOACTIVATE|SWP_SHOWWINDOW);
            InvalidateRect(self.toast,ptr::null(),0);
            KillTimer(self.toast,TIMER_TOAST);
            SetTimer(self.toast,TIMER_TOAST,1700,0);
        }
    }
    fn snip(&mut self) {
        // Capture is an external operation, not a reason to dismiss the flyout.
        // Register a foreground/clipboard lifecycle guard before launching the
        // documented image-capture overlay. Nothing is delayed or hidden.
        if self.capture_session.active() { return; }
        if self.visible {
            if let Err(error)=self.capture_session.begin(self.window) {
                self.capture_error(error);
                return;
            }
        }
        // Wait for the actual key-up event instead of delaying for a timer.
        // Never hide the flyout before invoking the native capture shortcut.
        match capture::request_capture_after_release(self.window, self.config.snip) {
            Ok(())=>{ self.status.clear(); },
            Err(error)=>{
                self.capture_session.finish(self.window);
                self.capture_error(error);
            },
        }
    }
    fn capture_error(&mut self, error:String) {
        self.status=error;
        self.toast_text="Screen snip unavailable".into();
        self.toast_symbol="!".into();
        // Capture errors should be visible even with the flyout hidden.
        self.show_toast();
        self.repaint();
    }
    fn add_hit(&mut self,r:Rect,action:Action) {self.hits.push(Hit{rect:r,action});}
    fn action(&mut self,action:Action,x:i32,_y:i32) {
        match action {
            Action::Settings=>self.go(Page::Settings),
            Action::Back=>{
                self.go(match self.page {
                    Page::Settings=>Page::Mixer,
                    Page::DeviceIcons=>Page::Devices,
                    _=>Page::Settings,
                });
            },
            Action::Switch=>self.next_device(),
            Action::Page(page)=>self.go(page),
            Action::DeviceToggle(i)=>{
                if let Some(a)=self.audio.as_ref() {
                    if let Some(d)=a.devices.get(i) {
                        let id=d.id.clone();
                        let selected=a.devices.iter().filter(|x|self.config.device_enabled(&x.id)).count();
                        let on=self.config.device_enabled(&id);
                        if on&&selected==1 {self.status="At least one device must remain enabled".into();}
                        else {self.config.enabled_devices.insert(id,!on);self.status.clear();let _=self.config.save();}
                        self.repaint();
                    }
                }
            },
            Action::DeviceRename(i)=>self.rename(i),
            Action::DeviceIcon(i)=>{
                self.icon_device=Some(i);
                self.go(Page::DeviceIcons);
            },
            Action::PickIcon(i)=>{
                if let Some(device) = self.icon_device.and_then(|ix|self.audio.as_ref().and_then(|a|a.devices.get(ix))) {
                    if let Some((style,_,_))=DEVICE_SYMBOLS.get(i) {
                        self.config.device_icons.insert(device.id.clone(),(*style).to_owned());
                        let _=self.config.save();
                    }
                }
                self.go(Page::Devices);
            },
            Action::SaveAlias=>self.save_alias(),
            Action::Capture(i)=>self.start_capture(i),
            Action::ToastToggle=>{self.config.toast_enabled=!self.config.toast_enabled;let _=self.config.save();self.repaint();},
            Action::StartupToggle=>self.toggle_startup(),
            Action::SetAccentMode(system)=>{
                if self.config.use_windows_accent!=system {
                    self.config.use_windows_accent=system;
                    let _=self.config.save();
                    self.palette=Palette::from_config(&self.config);
                    self.repaint();
                }
            },
            Action::StartupEnable=>self.finish_welcome(true),
            Action::StartupSkip=>self.finish_welcome(false),
            Action::TrayAbout=>{self.show();self.go(Page::About);},
            Action::TrayCheckUpdates=>{self.show();self.go(Page::About);self.begin_update_check();},
            Action::CheckUpdates=>self.begin_update_check(),
            Action::OpenUpdateRelease=>{
                if let UpdateStatus::Available{url,..}=&self.update_status {
                    let url=url.clone();self.open_url(&url);
                }
            },
            Action::OpenRepository=>{
                if let Some(url)=updates::repository_url() {self.open_url(&url);}
            },
            Action::OpenLicense=>self.open_url("https://opensource.org/license/mit"),
            Action::SetToastPosition(index)=>{
                self.config.toast_position=Some(index.min(2));
                let _=self.config.save();self.repaint();
            },
            Action::MasterMute=>{if let Some(a)=self.audio.as_mut(){a.toggle_master_mute();}self.repaint();},
            Action::AppMute(i)=>{if let Some(a)=self.audio.as_mut(){a.toggle_mute(i);}self.repaint();},
            Action::MasterSlider|Action::AppSlider(_)=>{
                self.tooltip_visible=false;
                unsafe { KillTimer(self.window,TIMER_APP_TOOLTIP); }
                self.dragging=Some(action);
                self.drag_slider(action,x);
                self.flush_slider();
            },
            Action::TrayOpen=>self.show(),
            Action::TraySettings=>{self.show();self.go(Page::Settings);},
            Action::TrayExit=>unsafe {DestroyWindow(self.toast);DestroyWindow(self.window);},
            Action::NextApps=>{
                let len=self.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                self.page_offset+=self.page_size(); if self.page_offset>=len {self.page_offset=0;}
                self.resize();self.repaint();
            },
        }
    }
    // The thumb follows the pointer immediately. Core Audio writes are coalesced
    // at ~60 Hz, preventing UI message starvation while moving the mouse fast.
    fn drag_slider(&mut self,action:Action,x:i32) {
        let value=((x-SLIDER_LEFT) as f32/(SLIDER_RIGHT-SLIDER_LEFT) as f32)
            .clamp(0.0,1.0);
        let same=self.pending_slider.map(|(a,v)| a==action && (v-value).abs()<0.001)
            .unwrap_or(false);
        if same { return; }
        self.pending_slider=Some((action,value));
        self.repaint();
        // UI paints are independent of the (potentially blocking) audio calls.
        let now=unsafe {GetTickCount()};
        if now.wrapping_sub(self.last_audio_send)>=17 { self.flush_slider(); }
        else if !self.slider_timer_armed {
            unsafe { SetTimer(self.window,TIMER_SLIDER,17,0); }
            self.slider_timer_armed=true;
        }
    }
    fn flush_slider(&mut self) {
        unsafe { KillTimer(self.window,TIMER_SLIDER); }
        self.slider_timer_armed=false;
        if let Some((action,value))=self.pending_slider.take() {
            if let Some(audio)=self.audio.as_mut() {
                match action {
                    Action::MasterSlider=>audio.set_master(value),
                    Action::AppSlider(i)=>audio.set_level(i,value),
                    _=>{},
                }
            }
            self.last_audio_send=unsafe {GetTickCount()};
            self.repaint();
        }
    }
    fn slider_value(&self,action:Action,actual:f32)->f32 {
        match self.pending_slider {
            Some((target,value)) if target==action =>value,
            _=>actual,
        }
    }
    fn rename(&mut self,i:usize) {
        let Some(d)=self.audio.as_ref().and_then(|a|a.devices.get(i)) else {return};
        let name=self.config.display_name(&d.id,&d.name).to_owned();
        self.edit_device=Some(i);
        unsafe {
            SetWindowTextW(self.edit,wide(&name).as_ptr());
            MoveWindow(self.edit,self.s(61),self.s(90+i as i32*58+9),self.s(WIDTH-167),self.s(31),1);
            ShowWindow(self.edit,SW_SHOWNORMAL);
            SetFocus(self.edit);
        }
        self.repaint();
    }
    fn save_alias(&mut self) {
        if let Some(i)=self.edit_device.take() {
            let mut text=[0u16;256];
            let len=unsafe {GetWindowTextW(self.edit,text.as_mut_ptr(),text.len() as i32).max(0)} as usize;
            if let Some(d)=self.audio.as_ref().and_then(|a|a.devices.get(i)) {
                let text=String::from_utf16_lossy(&text[..len]).trim().to_owned();
                if text.is_empty() {self.config.aliases.remove(&d.id);}
                else {self.config.aliases.insert(d.id.clone(),text);}
                let _=self.config.save();
            }
        }
        unsafe {ShowWindow(self.edit,SW_HIDE);SetFocus(self.window);}
        self.repaint();
    }
}

const DEVICE_SYMBOLS:[(&str,&str,&str);8]=[
    ("speaker","Speakers","\u{E7F5}"),
    ("headphones","Headphones","\u{E7F6}"),
    ("monitor","Monitor","\u{E7F4}"),
    ("bluetooth","Bluetooth","\u{E702}"),
    ("headset","Headset","\u{E95B}"),
    ("usb","USB","\u{E88E}"),
    ("audio","Audio","\u{E8D6}"),
    ("device","Device","\u{E70C}"),
];
fn icon_glyph(style:Option<&str>)->&'static str {
    let style=style.unwrap_or("speaker");
    DEVICE_SYMBOLS.iter().find(|x|x.0==style).map(|x|x.2).unwrap_or("\u{E7F5}")
}
fn device_glyph(config:&Config,id:&str,name:&str)->&'static str {
    if let Some(style)=config.device_icons.get(id) {return icon_glyph(Some(style.as_str()));}
    let lower=name.to_lowercase();
    if lower.contains("head") || lower.contains("pc38x") || lower.contains("ear") {
        icon_glyph(Some("headphones"))
    } else {icon_glyph(Some("speaker"))}
}

// --------- DPI-scaled, double-buffered Win32/GDI paint layer ---------
#[derive(Clone,Copy)]
struct Palette {
    accent:u32, background:u32, panel:u32, text:u32, dim:u32,
    divider:u32, rail:u32, thumb_halo:u32,
    activity:u32, brand:bool, high_contrast:bool,
}
impl Palette {
    fn from_config(config:&Config)->Self {
        let mut palette=Self::system();
        // High-contrast themes take precedence over cosmetic accent preferences.
        // Ordinary Windows light/dark mode must not change the flyout surface.
        palette.brand=!config.use_windows_accent && !palette.high_contrast;
        if palette.brand {
            palette.accent=BRAND_TEAL;
            palette.activity=BRAND_METER;
        } else if palette.high_contrast {
            palette.activity=palette.accent;
        } else {
            palette.activity=lighten_color(palette.accent,0.55);
        }
        palette
    }
    fn system()->Self {
        if let Some(contrast)=Self::high_contrast_palette() { return contrast; }
        let settings=UISettings::new().ok();
        let read=|which|settings.as_ref().and_then(|ui|ui.GetColorValue(which).ok());
        let accent=read(UIColorType::Accent)
            .map(|c|rgb(c.R,c.G,c.B))
            .unwrap_or_else(||{
                let mut argb=0; let mut opaque=0;
                if unsafe {DwmGetColorizationColor(&mut argb,&mut opaque)}>=0 {
                    rgb(((argb>>16)&255) as u8,((argb>>8)&255) as u8,(argb&255) as u8)
                } else {rgb(178,165,231)}
            });
        // The custom GDI+ flyout always uses its own opaque dark surface.
        // Switching Windows into Light Mode changes only the optional accent,
        // never text contrast, the header, or the background.
        Self { accent, background:rgb(29,30,33),panel:rgb(42,43,47),
            text:rgb(248,248,250),dim:rgb(171,172,178),divider:rgb(65,67,72),
            rail:rgb(100,102,108),thumb_halo:rgb(74,76,82),
            activity:lighten_color(accent,0.55),brand:false,high_contrast:false }
    }
    fn high_contrast_palette()->Option<Self> {
        let mut state=HighContrast {
            size:mem::size_of::<HighContrast>() as u32,
            flags:0,default_scheme:ptr::null_mut(),
        };
        if unsafe {SystemParametersInfoW(SPI_GETHIGHCONTRAST,state.size,
                &mut state as *mut _ as *mut c_void,0)}==0 ||
                state.flags & HCF_HIGHCONTRASTON == 0 {return None;}
        // System colors are required for readability with Windows Contrast
        // Themes, which may legitimately be light or dark.
        let background=unsafe {GetSysColor(5)}; // COLOR_WINDOW
        let panel=unsafe {GetSysColor(15)}; // COLOR_BTNFACE
        let text=unsafe {GetSysColor(8)}; // COLOR_WINDOWTEXT
        let accent=unsafe {GetSysColor(13)}; // COLOR_HIGHLIGHT
        let rail=unsafe {GetSysColor(17)}; // COLOR_GRAYTEXT
        Some(Self {accent,background,panel,text,dim:text,
            divider:rail,rail,thumb_halo:rail,
            activity:accent,brand:false,high_contrast:true})
    }
}
fn background_is_dark(color:u32)->bool {
    let sum=(color&255)+((color>>8)&255)+((color>>16)&255);
    sum < 384
}
fn lighten_color(color:u32,amount:f32)->u32 {
    let t=amount.clamp(0.0,1.0);
    let blend=|c:u32|((c as f32)*(1.0-t)+255.0*t).round() as u8;
    rgb(blend(color&255),blend((color>>8)&255),blend((color>>16)&255))
}
// Coordinates in logical 96-DPI units. Track positions are always within
// [SLIDER_LEFT, SLIDER_RIGHT], so a centered 40px value bubble fits the flyout
// at both endpoints without losing its visual attachment to the thumb.
fn value_bubble_rect(thumb_x:i32, track_y:i32)->Rect {
    Rect::new(thumb_x-20,track_y-42,40,27)
}

// With a 46px row pitch, this space is between the hovered track and the
// following icon. It does not obscure the control the user is adjusting.
fn tooltip_rect(row_y:i32,width:i32)->Rect {
    Rect::new(56,row_y+34,width,21)
}

struct Canvas {
    dc:H, dpi:u32, theme:Palette,
    base_font:H, small_font:H, bold_font:H, icon_font:H, large_icon_font:H, tiny_icon_font:H, title_font:H,
    smooth:Option<smooth::SmoothCanvas>,
}
impl Canvas {
    fn new(dc:H,dpi:u32,theme:Palette)->Self {
        let font=|name:&str,size:i32,weight:i32|unsafe{
            CreateFontW(-scaled(size,dpi),0,0,0,weight,0,0,0,1,0,0,5,0,wide(name).as_ptr())
        };
        unsafe {SetBkMode(dc,TRANSPARENT);}
        Self {dc,dpi,theme,base_font:font("Segoe UI Variable Text",12,400),
            small_font:font("Segoe UI Variable Text",11,400),
            bold_font:font("Segoe UI Variable Text",12,600),
            icon_font:font("Segoe Fluent Icons",18,400),
            large_icon_font:font("Segoe Fluent Icons",21,400),
            tiny_icon_font:font("Segoe Fluent Icons",11,400),
            title_font:font("Segoe UI Variable Display",15,650),
            smooth:smooth::SmoothCanvas::new(dc)}
    }
    fn s(&self,v:i32)->i32 {scaled(v,self.dpi)}
    fn box_color(&self,r:Rect,color:u32,radius:i32) {
        if let Some(g)=&self.smooth {
            g.rounded(self.s(r.left) as f32,self.s(r.top) as f32,
                self.s(r.width()) as f32,self.s(r.height()) as f32,
                self.s(radius) as f32,color);
            return;
        }
        unsafe {
            let b=CreateSolidBrush(color);let p=CreatePen(PS_SOLID,1,color);
            let old_b=SelectObject(self.dc,b);let old_p=SelectObject(self.dc,p);
            if radius>0 { RoundRect(self.dc,self.s(r.left),self.s(r.top),self.s(r.right),self.s(r.bottom),self.s(radius*2),self.s(radius*2)); }
            else {Rectangle(self.dc,self.s(r.left),self.s(r.top),self.s(r.right),self.s(r.bottom));}
            SelectObject(self.dc,old_p);SelectObject(self.dc,old_b);
            DeleteObject(p);DeleteObject(b);
        }
    }
    fn gradient_box(&self,r:Rect,first:u32,last:u32,radius:i32) {
        if r.width()<=0 || r.height()<=0 {return;}
        if let Some(g)=&self.smooth {
            g.gradient_rounded(self.s(r.left) as f32,self.s(r.top) as f32,
                self.s(r.width()) as f32,self.s(r.height()) as f32,
                self.s(radius) as f32,first,last);
        } else {self.box_color(r,first,radius);}
    }
    fn vertical_gradient_box(&self,r:Rect,top:u32,bottom:u32,radius:i32) {
        if let Some(g)=&self.smooth {
            g.gradient_rounded_vertical(
                self.s(r.left) as f32, self.s(r.top) as f32,
                self.s(r.width()) as f32, self.s(r.height()) as f32,
                self.s(radius) as f32, top, bottom,
            );
        } else { self.box_color(r,bottom,radius); }
    }
    fn alpha_box(&self,r:Rect,color:u32,opacity:u8,radius:i32) {
        if let Some(g)=&self.smooth {
            g.rounded_alpha(self.s(r.left) as f32,self.s(r.top) as f32,
                self.s(r.width()) as f32,self.s(r.height()) as f32,
                self.s(radius) as f32,color,opacity);
        } else {self.box_color(r,color,radius);}
    }
    fn line(&self,x:i32,y:i32,w:i32,color:u32) {self.box_color(Rect::new(x,y,w,1),color,0);}
    fn text(&self,value:&str,r:Rect,color:u32,font:u8,flags:u32) {
        if let Some(g)=&self.smooth {g.flush();}
        let wide=wide(value);let mut area=Rect::new(self.s(r.left),self.s(r.top),self.s(r.width()),self.s(r.height()));
        unsafe {
            let font=match font {1=>self.small_font,2=>self.bold_font,3=>self.icon_font,4=>self.tiny_icon_font,5=>self.title_font,6=>self.large_icon_font,_=>self.base_font};
            let old=SelectObject(self.dc,font);SetTextColor(self.dc,color);
            DrawTextW(self.dc,wide.as_ptr(),(wide.len()-1) as i32,&mut area,DT_SINGLELINE|DT_VCENTER|DT_END_ELLIPSIS|DT_NOPREFIX|flags);
            SelectObject(self.dc,old);
        }
    }
    fn measure_small_text(&self,value:&str)->i32 {
        if value.is_empty() {return 0;}
        let characters=wide(value);
        let mut bounds=Rect::new(0,0,self.s(2000),0);
        unsafe {
            let old=SelectObject(self.dc,self.small_font);
            DrawTextW(self.dc,characters.as_ptr(),(characters.len()-1) as i32,&mut bounds,
                DT_CALCRECT|DT_SINGLELINE|DT_NOPREFIX);
            SelectObject(self.dc,old);
        }
        ((bounds.width() as i64 * 96 + self.dpi as i64 - 1) / self.dpi as i64) as i32
    }
    fn app_tooltip(&self,label:&str,row_y:i32) {
        let max_width=WIDTH-74;
        let width=(self.measure_small_text(label)+26).clamp(58,max_width);
        let rect=tooltip_rect(row_y,width);
        // The tooltip sits between adjacent tracks, not on top of the slider.
        self.alpha_box(Rect::new(rect.left,rect.top+2,rect.width(),rect.height()),
            rgb(0,0,0),42,7);
        self.box_color(rect,self.theme.divider,7);
        self.box_color(Rect::new(rect.left+1,rect.top+1,rect.width()-2,rect.height()-2),
            self.theme.panel,6);
        self.text(label,Rect::new(rect.left+10,rect.top+1,rect.width()-20,rect.height()-2),
            self.theme.text,1,0);
    }
    fn symbol(&self,glyph:&str,r:Rect,color:u32) {self.text(glyph,r,color,3,DT_CENTER);}
    fn large_symbol(&self,glyph:&str,r:Rect,color:u32) {self.text(glyph,r,color,6,DT_CENTER);}
    fn small_symbol(&self,glyph:&str,r:Rect,color:u32) {self.text(glyph,r,color,4,DT_CENTER);}
    fn app_logo(&self, icon:H, cached_image:isize, x:i32, y:i32, size:i32) {
        // Draw the transparent 256px PArgb bitmap generated from the original
        // SVG. Avoid HICON conversion artefacts along fine internal edges.
        if let Some(g)=&self.smooth {
            if g.icon(cached_image,self.s(x),self.s(y),self.s(size),255) {return;}
            g.flush();
        }
        if icon!=0 {
            unsafe {DrawIconEx(self.dc,self.s(x),self.s(y),icon,self.s(size),self.s(size),0,0,0x0003);}
        }
    }
    fn native_icon(&self,icon:H,image:H,x:i32,y:i32,muted:bool) {
        if icon==0 {
            self.symbol("\u{E7F4}",Rect::new(x,y,24,24),self.theme.dim);
            return;
        }
        // Original Shell/EXE RGB artwork, smoothly downscaled from a 48/64px
        // source. Muting changes its alpha only, not its colors or glyph.
        let opacity=if muted {138} else {255};
        if let Some(g)=&self.smooth {
            if g.icon(image,self.s(x-1),self.s(y-1),self.s(28),opacity) {return;}
            g.flush();
        }
        // GDI+ is a standard Windows component, but retain a safe fallback.
        unsafe { DrawIconEx(self.dc,self.s(x-1),self.s(y-1),icon,self.s(28),self.s(28),0,0,0x0003); }
    }
    fn circle(&self,x:i32,y:i32,size:i32,color:u32) {
        if let Some(g)=&self.smooth {
            g.circle(self.s(x) as f32-self.s(size) as f32/2.0,
                self.s(y) as f32-self.s(size) as f32/2.0,
                self.s(size) as f32,color);
            return;
        }
        unsafe {
            let brush=CreateSolidBrush(color);
            let pen=CreatePen(PS_SOLID,1,color);
            let old_brush=SelectObject(self.dc,brush);
            let old_pen=SelectObject(self.dc,pen);
            Ellipse(self.dc,self.s(x-size/2),self.s(y-size/2),
                self.s(x+(size+1)/2),self.s(y+(size+1)/2));
            SelectObject(self.dc,old_pen);
            SelectObject(self.dc,old_brush);
            DeleteObject(pen);
            DeleteObject(brush);
        }
    }
    fn slider(&self,y:i32,level:f32,muted:bool,peak:f32,hover:bool,pressed:bool) {
        let length=SLIDER_RIGHT-SLIDER_LEFT;
        let progress=slider_progress(level,length);
        // The gradient belongs to the whole track, not to the currently
        // filled segment. This prevents its colors changing as the thumb moves.
        let color=if muted {if self.theme.high_contrast {self.theme.accent}else{MUTED_RED}}
            else if self.theme.brand {brand_color_at(level)}
            else {self.theme.accent};
        let track_y=y-3;
        self.box_color(Rect::new(SLIDER_LEFT,track_y,length,6),self.theme.rail,3);
        if progress>0 {
            if self.theme.brand && !muted {
                self.brand_slider_fill(track_y,progress,length);
            } else {
                self.box_color(Rect::new(SLIDER_LEFT,track_y,progress,6),color,3);
            }
        }
        // Dashed peak activity is an estimate after the session's volume gain.
        // Also cap it at the CURRENT thumb position while dragging, before
        // asynchronous Core Audio sampling has processed the new level.
        // Dash height is decorative; it is not a frequency-spectrum graph.
        let meter_y=activity_lane_y(y);
        let live_peak=displayed_activity_peak(peak,level,muted);
        let activity_width=activity_length(live_peak,length);
        if live_peak>=0.012 && activity_width>=3 {
            let live_tint=if self.theme.brand {BRAND_METER}
                else {lighten_color(self.theme.accent,0.38)};
            let drawn=self.smooth.as_ref().is_some_and(|g| g.activity_dashes(
                self.s(SLIDER_LEFT) as f32,self.s(meter_y) as f32,
                self.s(length) as f32,live_peak,live_tint,225));
            if !drawn {
                for x in visualizer_bar_offsets(live_peak,length) {
                    self.box_color(Rect::new(SLIDER_LEFT+x,meter_y,3,2),live_tint,1);
                }
            }
        }
        let x=SLIDER_LEFT+progress;
        if hover || pressed {
            self.alpha_box(Rect::new(x-12,y-12,24,24),color,
                if pressed {20}else{24},12);
        }
        // The inner dot contracts on press, like a native Fluent slider.
        let (ring,core)=if pressed {(18,6)} else if hover {(20,9)} else {(17,9)};
        self.circle(x,y,ring,self.theme.thumb_halo);
        self.circle(x,y,core,color);
        if pressed {
            let amount=format!("{}",(level.clamp(0.0,1.0)*100.0).round() as u32);
            let bubble=value_bubble_rect(x,y);
            self.alpha_box(Rect::new(bubble.left,bubble.top+2,bubble.width(),bubble.height()),
                rgb(0,0,0),51,8);
            self.box_color(bubble,self.theme.divider,8);
            self.box_color(Rect::new(bubble.left+1,bubble.top+1,bubble.width()-2,bubble.height()-2),
                self.theme.panel,7);
            self.text(&amount,Rect::new(bubble.left+2,bubble.top+1,bubble.width()-4,bubble.height()-2),
                self.theme.text,2,DT_CENTER);
        }
    }
    fn brand_gradient_box(&self, rect:Rect, radius:i32) {
        if let Some(g)=&self.smooth {
            if g.gradient_rounded_fixed_stops(
                self.s(rect.left) as f32,self.s(rect.top) as f32,
                self.s(rect.width()) as f32,self.s(rect.width()) as f32,
                self.s(rect.height()) as f32,self.s(radius) as f32,
                brand::COLORS,brand::STOPS) {return;}
        }
        self.gradient_box(rect,BRAND_GREEN,BRAND_DEEP_BLUE,radius);
    }
    fn brand_slider_fill(&self,track_y:i32,filled:i32,full_length:i32) {
        if filled<=0 || full_length<=0 {return;}
        // A single GDI+ brush spans the FULL track. Four SVG-matching color stops
        // are rendered through ONE rounded fill path, avoiding the visible
        // join that two adjacent rounded gradient rectangles used to create.
        // The palette stays anchored when the volume thumb moves.
        if let Some(g)=&self.smooth {
            if g.gradient_rounded_fixed_stops(
                self.s(SLIDER_LEFT) as f32,self.s(track_y) as f32,
                self.s(filled) as f32,self.s(full_length) as f32,
                self.s(6) as f32,self.s(3) as f32,
                brand::COLORS, brand::STOPS) {return;}
        }
        // Safe rendering fallback: one seamless gradient, never two joined
        // segments, even if a particular Windows GDI+ call is unavailable.
        let last=brand_color_at(filled as f32/full_length as f32);
        self.gradient_box(Rect::new(SLIDER_LEFT,track_y,filled,6),BRAND_GREEN,last,3);
    }
    fn section(&self,name:&str,y:i32) {
        self.text(name,Rect::new(18,y,WIDTH-36,23),self.theme.text,2,0);
    }
    fn action_row(&self,y:i32,glyph:&str,title:&str,description:&str,tail:&str,hover:bool) {
        if hover { self.box_color(Rect::new(15,y,WIDTH-30,61),self.theme.panel,9); }
        self.alpha_box(Rect::new(23,y+13,32,32),self.theme.accent,18,9);
        self.symbol(glyph,Rect::new(29,y+17,20,23),self.theme.accent);
        self.text(title,Rect::new(69,y+5,WIDTH-118,25),self.theme.text,2,0);
        self.text(description,Rect::new(69,y+29,WIDTH-118,21),self.theme.dim,1,0);
        self.symbol(tail,Rect::new(WIDTH-44,y+19,18,20),self.theme.dim);
        self.line(69,y+61,WIDTH-99,self.theme.divider);
    }

}
impl Drop for Canvas {
    fn drop(&mut self) {unsafe{DeleteObject(self.base_font);DeleteObject(self.small_font);DeleteObject(self.bold_font);DeleteObject(self.icon_font);DeleteObject(self.large_icon_font);DeleteObject(self.tiny_icon_font);DeleteObject(self.title_font);}}
}
// Fixed brand palette from the SwiftTray logo. Windows accent remains selectable.
// Win32 COLORREF stores colors as 0x00BBGGRR.
const MUTED_RED: u32 = 0x00504BE3; // RGB(227,75,80)

fn mix_color(first:u32,second:u32,t:f32)->u32 {
    let t=t.clamp(0.0,1.0);
    let mix=|shift:u32| -> u32 {
        let a=((first>>shift)&255) as f32;
        let b=((second>>shift)&255) as f32;
        (((a+(b-a)*t).round() as u32).min(255)) << shift
    };
    mix(0)|mix(8)|mix(16)
}

fn brand_color_at(level:f32)->u32 {
    brand::color_at(level)
}

fn slider_progress(level:f32,length:i32)->i32 {
    (level.clamp(0.0,1.0)*length.max(0) as f32).round() as i32
}
fn activity_length(peak:f32,length:i32)->i32 {
    slider_progress(peak,length)
}
// Enforce the knob's bound during live interaction, without requiring another
// Core Audio call on the rendering path.
fn displayed_activity_peak(peak:f32,level:f32,muted:bool)->f32 {
    if muted { return 0.0; }
    let volume=if level.is_finite() { level.clamp(0.0,1.0) } else {0.0};
    let activity=if peak.is_finite() { peak.clamp(0.0,1.0) } else {0.0};
    activity.min(volume)
}
// The visualizer lives 7 DIP below the volume track; it never overlays the thumb.
fn activity_lane_y(thumb_y:i32)->i32 {thumb_y+10}

fn visualizer_bar_offsets(peak:f32, track_width:i32) -> Vec<i32> {
    const PITCH:i32=6;
    const WIDTH:i32=3;
    let active=activity_length(peak,track_width);
    if active < WIDTH {return Vec::new();}
    (0..track_width.max(0)/PITCH+1)
        .map(|i|i*PITCH)
        .take_while(|&offset|offset+WIDTH<=active)
        .collect()
}

// Shared opaque surface: toast and flyout have identical corner, border,
// vertical tint and header treatment. No global alpha is applied to icons
// or typography, and wallpaper detail never bleeds through the content.
fn draw_surface(c:&Canvas,width:i32,height:i32) {
    c.box_color(Rect::new(0,0,width,height),c.theme.divider,11);
    if c.theme.high_contrast {
        c.box_color(Rect::new(1,1,width-2,height-2),c.theme.background,10);
    } else {
        let top=mix_color(c.theme.background,c.theme.panel,0.20);
        c.vertical_gradient_box(Rect::new(1,1,width-2,height-2),top,c.theme.background,10);
        c.alpha_box(Rect::new(2,2,width-4,50),c.theme.panel,74,9);
        c.alpha_box(Rect::new(15,2,width-30,1),c.theme.text,13,0);
    }
}

fn output_notch(c:&Canvas,x:i32,y:i32) {
    if c.theme.brand {
        c.brand_gradient_box(Rect::new(x,y,3,18),2);
    } else {
        c.box_color(Rect::new(x,y,3,18),c.theme.accent,2);
    }
}

fn draw_app(app:&mut App,c:&Canvas) {
    let a=c.theme.accent; app.hits.clear();
    draw_surface(c,app.page_width(),app.page_height());
    match app.page {
        Page::Mixer=>draw_mixer(app,c,a),
        Page::Settings=>draw_settings(app,c),
        Page::Appearance=>draw_appearance(app,c),
        Page::Devices=>draw_devices(app,c,a),
        Page::DeviceIcons=>draw_device_icons(app,c,a),
        Page::Shortcuts=>draw_shortcuts(app,c,a),
        Page::Notifications=>draw_notifications(app,c,a),
        Page::About=>draw_about(app,c),
        Page::Welcome=>draw_welcome(app,c,a),
        Page::TrayMenu=>draw_tray_menu(app,c),
    }
}
fn header(app:&mut App,c:&Canvas,title:&str) {
    if app.hover==Some(Action::Back) {c.box_color(Rect::new(13,11,37,35),c.theme.panel,8);}
    c.symbol("\u{E72B}",Rect::new(20,14,22,28),c.theme.text);
    c.text(title,Rect::new(57,12,WIDTH-87,31),c.theme.text,2,0);
    app.add_hit(Rect::new(13,10,37,37),Action::Back);
    c.line(17,53,WIDTH-34,c.theme.divider);
}
fn clean_audio_label(name:&str)->String {
    let trimmed=name.trim();
    if trimmed.eq_ignore_ascii_case("system sounds") {return "System sounds".to_owned();}
    if trimmed.is_empty() {return "Application".to_owned();}
    trimmed.to_owned()
}
fn draw_mixer(app:&mut App,c:&Canvas,_a:u32) {
    let (name,device_icon)=app.audio.as_ref().and_then(|x|x.current_device())
        .map(|d|(app.config.display_name(&d.id,&d.name).to_owned(),
            device_glyph(&app.config,&d.id,&d.name)))
        .unwrap_or_else(||("No audio device".into(),"\u{E7F5}"));

    // Identical active-output indicator in the mixer and switch toast.
    output_notch(c,14,20);
    c.large_symbol(device_icon,Rect::new(22,14,26,29),c.theme.accent);
    c.text(&name,Rect::new(55,8,WIDTH-162,40),c.theme.text,5,0);
    if app.hover==Some(Action::Switch) {
        c.box_color(Rect::new(WIDTH-91,10,38,34),c.theme.panel,7);
    }
    c.symbol("\u{E8AB}",Rect::new(WIDTH-88,12,31,30),c.theme.accent);
    app.add_hit(Rect::new(WIDTH-91,8,39,37),Action::Switch);
    if app.hover==Some(Action::Settings) {
        c.box_color(Rect::new(WIDTH-49,10,35,34),c.theme.panel,7);
    }
    c.symbol("\u{E713}",Rect::new(WIDTH-47,12,31,30),c.theme.text);
    app.add_hit(Rect::new(WIDTH-51,8,43,37),Action::Settings);

    // The output header is visually distinct by its surface alone. Avoid a
    // redundant horizontal rule; shift the section and every slider up 5 DIP.
    c.section("Volume mixer",60);

    let first=MIXER_FIRST;
    let snapshot:Vec<_>=app.audio.as_ref().map(|x|x.sessions.iter().enumerate()
        .skip(app.page_offset).take(app.page_size())
        .map(|(i,s)|(i,s.app_user_model_id.clone(),s.session_icon.clone(),s.executable.clone(),s.level,s.muted,s.peak,s.is_system_sounds))
        .collect()).unwrap_or_default();
    let (master_level,master_mute,master_peak)=app.audio.as_ref()
        .map(|x|(x.master_level,x.master_muted,x.master_peak)).unwrap_or((0.0,false,0.0));
    mixer_row(app,c,first,master_level,master_mute,master_peak,0,0,0,true,false);
    for (row,(i,app_id,icon,exe,level,mute,peak,is_system_sounds)) in snapshot.iter().enumerate() {
        // Packaged apps use their actual Windows Shell icons, not a generic
        // WebView2 executable icon. Unpackaged games use their native EXE icon.
        let (native,raster)=if *is_system_sounds {(0,0)}else{
            let handle=app.icons.icon_for(app_id.as_deref(),icon.as_deref(),exe.as_deref());
            (handle,app.icons.raster_for(handle))
        };
        mixer_row(app,c,first+((row+1) as i32)*ROW_HEIGHT,*level,*mute,*peak,
            native,raster,*i,false,*is_system_sounds);
    }
    if app.audio.is_none() {
        c.text("Audio engine unavailable",Rect::new(22,first+53,WIDTH-42,24),c.theme.dim,1,0);
    }
    if app.tooltip_visible && app.dragging.is_none() {
        if let Some(action)=app.hover {
            let target=match action {
                Action::MasterMute => Some((first,"System volume".to_owned())),
                Action::AppMute(index) if index >= app.page_offset && index < app.page_offset+app.page_size() => {
                    app.audio.as_ref().and_then(|a|a.sessions.get(index)).map(|session| {
                        (first+((index-app.page_offset+1) as i32)*ROW_HEIGHT,session.name.clone())
                    })
                },
                _=>None,
            };
            if let Some((y,label))=target {
                c.app_tooltip(&clean_audio_label(&label),y);
            }
        }
    }
    let count=app.audio.as_ref().map(|x|x.sessions.len()).unwrap_or(0);
    if count>app.page_size() {
        let y=first+(snapshot.len() as i32+1)*ROW_HEIGHT;
        let remaining=count.saturating_sub((app.page_offset+app.page_size()).min(count));
        c.text(&if remaining==0 {"Back to first apps".to_owned()} else {format!("{remaining} more apps")},
            Rect::new(20,y+2,WIDTH-80,29),c.theme.accent,1,0);
        c.symbol("\u{E76C}",Rect::new(WIDTH-49,y+3,23,25),c.theme.accent);
        app.add_hit(Rect::new(16,y,WIDTH-32,35),Action::NextApps);
    }
}
fn mixer_row(app:&mut App,c:&Canvas,y:i32,level:f32,muted:bool,peak:f32,
    icon:H,raster:H,index:usize,master:bool,system_sounds:bool) {
    let icon_action=if master {Action::MasterMute}else{Action::AppMute(index)};
    let slider_action=if master {Action::MasterSlider}else{Action::AppSlider(index)};
    let visually_muted=muted || app.slider_value(slider_action,level)<=f32::EPSILON;
    let hover_icon=app.hover==Some(icon_action);
    if hover_icon {
        // A gentle rollover cue, not a square block around native app icons.
        c.alpha_box(Rect::new(15,y+7,33,32),c.theme.accent,12,9);
    }
    if master {
        c.symbol(crate::volume_glyph::for_level(level,visually_muted),
            Rect::new(17,y+9,27,25),if visually_muted {MUTED_RED}else{c.theme.text});
    } else {
        if system_sounds {
            // Fluent audio glyph for the Windows sound-effects session.
            c.large_symbol("\u{E8D6}",Rect::new(18,y+8,27,28),c.theme.text);
        } else {
            c.native_icon(icon,raster,19,y+10,visually_muted);
        }
        if visually_muted {
            // The native app icon remains completely untouched. A clean,
            // separate muted indicator is anchored below-right of it.
            // Restore the v0.14 muted-app indicator, without modifying the icon.
            c.box_color(Rect::new(42,y+25,19,20),c.theme.background,6);
            c.symbol("\u{E74F}",Rect::new(43,y+26,18,19),MUTED_RED);
        }
    }
    let actual=app.slider_value(slider_action,level);
    c.slider(y+22,actual,visually_muted,peak,
        app.hover==Some(slider_action)||app.dragging==Some(slider_action),
        app.dragging==Some(slider_action));
    app.add_hit(Rect::new(12,y+4,46,36),icon_action);
    app.add_hit(Rect::new(SLIDER_LEFT-4,y+1,SLIDER_RIGHT-SLIDER_LEFT+16,42),slider_action);
}
fn draw_settings(app:&mut App,c:&Canvas) {
    header(app,c,"Settings");
    // Small section labels establish hierarchy without adding heavy cards.
    c.text("QUICK SETTINGS",Rect::new(23,59,WIDTH-46,20),c.theme.dim,1,0);
    let pages=[
        (Page::Devices,"\u{E767}","Audio devices","Switch outputs and rename devices"),
        (Page::Shortcuts,"\u{E765}","Shortcuts","Choose your keyboard shortcuts"),
        (Page::Notifications,"\u{E7F4}","Notifications","Customize the output popup"),
        (Page::Appearance,"\u{E790}","Appearance","SwiftTray or Windows accent"),
    ];
    for (i,(page,glyph,name,description)) in pages.into_iter().enumerate() {
        let y=82+i as i32*61;
        let active=app.hover==Some(Action::Page(page));
        // Keep the clean v0.16 rows: only hover gets a raised surface.
        if active {
            c.box_color(Rect::new(16,y,WIDTH-32,55),c.theme.panel,9);
            c.box_color(Rect::new(17,y+14,3,27),c.theme.accent,2);
        }
        c.alpha_box(Rect::new(25,y+9,36,36),c.theme.accent,20,10);
        c.symbol(glyph,Rect::new(30,y+14,26,25),c.theme.accent);
        c.text(name,Rect::new(73,y+3,WIDTH-124,26),c.theme.text,2,0);
        c.text(description,Rect::new(73,y+28,WIDTH-125,21),c.theme.dim,1,0);
        c.symbol("\u{E76C}",Rect::new(WIDTH-41,y+14,20,25),
            if active {c.theme.accent}else{c.theme.dim});
        app.add_hit(Rect::new(16,y,WIDTH-32,56),Action::Page(page));
    }
    c.line(23,331,WIDTH-46,c.theme.divider);
    c.text("GENERAL",Rect::new(23,337,WIDTH-46,19),c.theme.dim,1,0);
    let startup_y=359;
    if app.hover==Some(Action::StartupToggle) {
        c.box_color(Rect::new(16,startup_y,WIDTH-32,55),c.theme.panel,9);
    }
    c.alpha_box(Rect::new(25,startup_y+10,36,36),c.theme.accent,15,10);
    c.symbol("\u{E768}",Rect::new(31,startup_y+15,24,26),c.theme.accent);
    c.text("Launch at startup",Rect::new(73,startup_y+2,WIDTH-157,26),c.theme.text,2,0);
    c.text("Start when you sign in",Rect::new(73,startup_y+27,WIDTH-151,21),c.theme.dim,1,0);
    toggle(c,app.startup_on,WIDTH-71,startup_y+16,c.theme.accent);
    app.add_hit(Rect::new(16,startup_y,WIDTH-32,55),Action::StartupToggle);
    if !app.startup_error.is_empty() {
        c.text(&app.startup_error,Rect::new(24,411,WIDTH-48,17),MUTED_RED,1,0);
    }
    let about_y=429;
    if app.hover==Some(Action::Page(Page::About)) {
        c.box_color(Rect::new(16,about_y,WIDTH-32,39),c.theme.panel,8);
    }
    c.symbol("\u{E946}",Rect::new(33,about_y+8,20,23),c.theme.dim);
    c.text("About",Rect::new(73,about_y+6,WIDTH-119,27),c.theme.text,0,0);
    c.symbol("\u{E76C}",Rect::new(WIDTH-41,about_y+7,20,24),c.theme.dim);
    app.add_hit(Rect::new(16,about_y,WIDTH-32,39),Action::Page(Page::About));
}

fn draw_appearance(app:&mut App,c:&Canvas) {
    header(app,c,"Appearance");
    c.text("ACCENT COLOR",Rect::new(24,61,WIDTH-48,21),c.theme.dim,1,0);
    c.text("Choose how SwiftTray colors its controls.",
        Rect::new(24,83,WIDTH-48,21),c.theme.dim,1,0);
    let options=[(false,"SwiftTray","Mint to teal, inspired by the app logo"),
                 (true,"Windows accent","Match your Windows personalization")];
    for (i,(system,title,desc)) in options.iter().enumerate() {
        let y=109+i as i32*66;
        let selected=app.config.use_windows_accent==*system;
        let hovered=app.hover==Some(Action::SetAccentMode(*system));
        let r=Rect::new(17,y,WIDTH-34,61);
        if selected||hovered {c.box_color(r,c.theme.panel,10);}
        if selected {
            if c.theme.brand {c.brand_gradient_box(Rect::new(18,y+11,3,39),2);}
            else {c.box_color(Rect::new(18,y+11,3,39),c.theme.accent,2);}
        }
        let glyph=if selected {"\u{E73E}"}else{"\u{E739}"};
        c.symbol(glyph,Rect::new(26,y+16,22,28),if selected {c.theme.accent}else{c.theme.dim});
        c.text(title,Rect::new(57,y+7,WIDTH-85,25),c.theme.text,2,0);
        c.text(desc,Rect::new(57,y+32,WIDTH-86,21),c.theme.dim,1,0);
        if !*system {
            c.brand_gradient_box(Rect::new(WIDTH-77,y+14,13,4),2);
            c.box_color(Rect::new(WIDTH-61,y+14,4,4),BRAND_DEEP_BLUE,2);
        }
        app.add_hit(r,Action::SetAccentMode(*system));
    }
    c.line(24,246,WIDTH-48,c.theme.divider);
    c.text("PREVIEW",Rect::new(24,253,WIDTH-48,21),c.theme.dim,1,0);
    c.box_color(Rect::new(17,274,WIDTH-34,49),c.theme.panel,9);
    c.symbol("\u{E767}",Rect::new(30,285,22,26),c.theme.accent);
    let length=WIDTH-130;
    c.box_color(Rect::new(72,296,length,4),c.theme.rail,2);
    if c.theme.brand {
        c.brand_gradient_box(Rect::new(72,296,length*7/10,4),2);
    } else {c.box_color(Rect::new(72,296,length*7/10,4),c.theme.accent,2);}
    c.circle(72+length*7/10,298,18,c.theme.thumb_halo);
    c.circle(72+length*7/10,298,10,c.theme.accent);
}

fn draw_devices(app:&mut App,c:&Canvas,a:u32) {
    header(app,c,"Audio devices");
    c.text("Choose the outputs used by Switch",Rect::new(23,57,WIDTH-46,26),c.theme.dim,1,0);
    let snapshot:Vec<_>=app.audio.as_ref().map(|x|x.devices.iter().take(6).enumerate()
        .map(|(i,d)|(i,d.id.clone(),app.config.display_name(&d.id,&d.name).to_owned(),
            app.config.device_enabled(&d.id),d.current)).collect()).unwrap_or_default();
    for (j,(i,id,name,enabled,current)) in snapshot.iter().enumerate() {
        let y=90+(j as i32)*58;
        if *current || app.hover==Some(Action::DeviceToggle(*i)) || app.hover==Some(Action::DeviceRename(*i)) {
            c.box_color(Rect::new(15,y,WIDTH-30,54),c.theme.panel,7);
        }
        if *current {c.box_color(Rect::new(16,y+15,3,24),a,1);}
        c.box_color(Rect::new(28,y+16,18,18),if *enabled {a}else{c.theme.rail},5);
        if *enabled {c.symbol("\u{E73E}",Rect::new(27,y+14,20,22),c.theme.background);}
        let editing=app.edit_device==Some(*i);
        if editing {
            c.box_color(Rect::new(56,y+6,WIDTH-159,36),c.theme.panel,7);
            c.box_color(Rect::new(WIDTH-88,y+7,61,35),a,7);
            c.text("Save",Rect::new(WIDTH-88,y+9,61,30),c.theme.background,2,DT_CENTER);
            app.add_hit(Rect::new(WIDTH-89,y+5,64,39),Action::SaveAlias);
        } else {
            c.text(name,Rect::new(58,y+5,WIDTH-142,23),c.theme.text,if *current {2}else{0},0);
            c.text(if *current {"Current output · Click to rename"}else{"Click to rename"},
                Rect::new(58,y+28,WIDTH-144,19),c.theme.dim,1,0);
            let glyph=device_glyph(&app.config,id,name);
            c.symbol(glyph,Rect::new(WIDTH-61,y+12,29,29),a);
            app.add_hit(Rect::new(55,y+3,WIDTH-137,47),Action::DeviceRename(*i));
            app.add_hit(Rect::new(WIDTH-73,y+3,53,47),Action::DeviceIcon(*i));
        }
        c.line(57,y+55,WIDTH-81,c.theme.divider);
        app.add_hit(Rect::new(20,y+3,35,47),Action::DeviceToggle(*i));
    }
    // Audio devices is limited to switch selection, aliases and icon choice.
    c.text("Click a device icon to customize its popup symbol",
        Rect::new(23,app.page_height()-39,WIDTH-46,24),c.theme.dim,1,0);
}
fn draw_device_icons(app:&mut App,c:&Canvas,a:u32) {
    header(app,c,"Choose device icon");
    let (id,name)=app.icon_device
        .and_then(|i|app.audio.as_ref().and_then(|x|x.devices.get(i)))
        .map(|d|(d.id.clone(),app.config.display_name(&d.id,&d.name).to_owned()))
        .unwrap_or_else(||(String::new(),"Audio output".into()));
    c.text(&name,Rect::new(24,60,WIDTH-48,28),c.theme.dim,1,0);
    let selected=app.config.device_icons.get(&id).cloned().unwrap_or_else(|| {
        if name.to_ascii_lowercase().contains("head") {"headphones".to_owned()}else{"speaker".to_owned()}
    });
    for (i,(style,label,glyph)) in DEVICE_SYMBOLS.iter().enumerate() {
        let row=(i/4) as i32;
        let col=(i%4) as i32;
        let x=18+col*82;
        let y=99+row*87;
        let rect=Rect::new(x,y,76,79);
        let active=*style==selected.as_str();
        let hovering=app.hover==Some(Action::PickIcon(i));
        if active||hovering {c.box_color(rect,c.theme.panel,9);}
        if active {c.box_color(Rect::new(x+10,y+7,56,3),a,2);}
        c.symbol(glyph,Rect::new(x+21,y+19,34,29),if active {a}else{c.theme.text});
        c.text(label,Rect::new(x+3,y+51,70,20),
            if active {a}else{c.theme.dim},1,DT_CENTER);
        app.add_hit(rect,Action::PickIcon(i));
    }
    c.text("Icons follow the Windows accent color",
        Rect::new(24,281,WIDTH-48,18),c.theme.dim,1,0);
}
fn draw_shortcuts(app:&mut App,c:&Canvas,a:u32) {
    header(app,c,"Shortcuts");
    c.text("Select an action to record a shortcut",Rect::new(23,58,WIDTH-46,25),c.theme.dim,1,0);
    let shortcuts=[("Open SwiftTray","Quick audio mixer","\u{E767}",app.config.popup),
        ("Switch output","Cycle selected devices","\u{E8AB}",app.config.switch),
        ("Screen snip","Windows capture overlay","\u{E8C6}",app.config.snip)];
    for (i,(name,desc,glyph,shortcut)) in shortcuts.iter().enumerate() {
        let y=86+(i as i32)*75;
        let listening=app.capture_shortcut==Some(i);
        let hover=app.hover==Some(Action::Capture(i));
        if listening {c.alpha_box(Rect::new(14,y,WIDTH-28,70),a,16,9);}
        else if hover {c.alpha_box(Rect::new(14,y,WIDTH-28,70),c.theme.text,9,9);}
        if listening {c.box_color(Rect::new(16,y+10,2,43),a,1);}
        c.alpha_box(Rect::new(21,y+9,31,32),a,16,8);
        c.symbol(glyph,Rect::new(25,y+13,23,24),a);
        c.text(name,Rect::new(61,y+2,157,25),c.theme.text,2,0);
        c.text(desc,Rect::new(61,y+27,WIDTH-88,20),c.theme.dim,1,0);
        // A light, underlined command instead of faux keyboard keycaps.
        // The underline and pencil appear on hover, not a permanent pill.
        let label=if listening {"Press keys...".to_owned()}else{shortcut.label()};
        let right=if hover && !listening {WIDTH-51}else{WIDTH-27};
        let width=c.measure_small_text(&label).clamp(36,136);
        let left=right-width;
        let shown=if listening || hover {a}else{c.theme.text};
        c.text(&label,Rect::new(left,y+2,width+1,29),shown,1,DT_RIGHT);
        if hover || listening {
            c.line(left,y+33,width,if listening {a}else{c.theme.divider});
            if hover && !listening {
                c.small_symbol("\u{E70F}",Rect::new(WIDTH-43,y+7,18,20),a);
            }
        }
        if listening {
            c.text("Esc to cancel",Rect::new(61,y+49,WIDTH-88,17),a,1,0);
        } else if let Some(error)=app.shortcut_errors[i].as_ref() {
            c.text(error,Rect::new(61,y+49,WIDTH-89,18),
                if error.starts_with("A single letter") {c.theme.dim}else{MUTED_RED},1,0);
        } else if app.capture_shortcut.is_none() && !app.hotkey_status[i] {
            c.text("Shortcut unavailable - click to change",
                Rect::new(61,y+49,WIDTH-89,18),MUTED_RED,1,0);
        }
        c.line(24,y+72,WIDTH-48,c.theme.divider);
        app.add_hit(Rect::new(16,y,WIDTH-32,71),Action::Capture(i));
    }
}
fn toggle(c:&Canvas,on:bool,x:i32,y:i32,a:u32) {
    c.box_color(Rect::new(x,y,41,22),if on {a}else{c.theme.rail},11);
    c.box_color(Rect::new(x+if on {21}else{3},y+3,16,16),if on {c.theme.background}else{c.theme.text},8);
}
fn draw_notifications(app:&mut App,c:&Canvas,a:u32) {
    header(app,c,"Notifications");
    c.text("AUDIO OUTPUT",Rect::new(24,61,WIDTH-48,22),c.theme.dim,1,0);
    let on=app.config.toast_enabled;
    let hover=app.hover==Some(Action::ToastToggle);
    if hover {c.box_color(Rect::new(15,91,WIDTH-30,64),c.theme.panel,9);}
    c.text("Show device popup",Rect::new(25,96,WIDTH-118,26),c.theme.text,2,0);
    c.text("Show the new output after switching",Rect::new(25,122,WIDTH-119,21),c.theme.dim,1,0);
    toggle(c,on,WIDTH-74,113,a);
    app.add_hit(Rect::new(16,90,WIDTH-32,64),Action::ToastToggle);
    c.line(24,159,WIDTH-48,c.theme.divider);
    c.text("Popup placement",Rect::new(25,170,WIDTH-50,25),c.theme.text,2,0);
    c.text("Choose a screen position",Rect::new(25,195,WIDTH-50,21),c.theme.dim,1,0);
    let labels=["Top right","Top left","Above flyout"];
    for (i,label) in labels.iter().enumerate() {
        let x=17+(i as i32)*109;
        let y=223;
        let current=app.config.popup_position()==i as u8;
        let hovered=app.hover==Some(Action::SetToastPosition(i as u8));
        c.box_color(Rect::new(x,y,105,73),if current||hovered {c.theme.panel}else{c.theme.background},9);
        if current {c.box_color(Rect::new(x+9,y+5,87,3),a,2);}
        // Small monitor preview; the accent rectangle shows the selected position.
        c.box_color(Rect::new(x+24,y+13,57,34),c.theme.rail,5);
        c.box_color(Rect::new(x+25,y+14,55,31),c.theme.background,4);
        c.box_color(Rect::new(x+26,y+41,53,3),c.theme.divider,1);
        match i {
            0=>c.box_color(Rect::new(x+61,y+18,15,7),a,2),
            1=>c.box_color(Rect::new(x+29,y+18,15,7),a,2),
            _=>{
                c.box_color(Rect::new(x+50,y+33,25,8),c.theme.dim,2);
                c.box_color(Rect::new(x+51,y+22,23,7),a,2);
            },
        }
        c.text(label,Rect::new(x+3,y+48,99,21),
            if current {c.theme.text}else{c.theme.dim},1,DT_CENTER);
        app.add_hit(Rect::new(x,y,105,73),Action::SetToastPosition(i as u8));
    }
}
// Compact, uncluttered About page. Keep all information visible at 100% DPI,
// and let the existing DPI-aware painter scale the controls together.
fn friendly_update_error(message:&str)->&'static str {
    if message.contains("No published GitHub Release") {
        "No published release yet"
    } else if message.contains("HTTP 403") || message.contains("HTTP 429") {
        "GitHub rate limit reached. Try later."
    } else if message.contains("unsupported version") || message.contains("invalid version") {
        "Release version could not be read"
    } else {
        "Could not check updates. Try again."
    }
}

fn draw_about(app:&mut App,c:&Canvas) {
    header(app,c,"About");
    // Use the embedded high-resolution original icon, without a tile or panel.
    let logo=if app.about_icon!=0 {app.about_icon}else{app.tray_icon};
    c.app_logo(logo,if app.about_image!=0 {app.about_image}else{app.tray_image},
        (WIDTH-ABOUT_LOGO_SIZE)/2,ABOUT_LOGO_TOP,ABOUT_LOGO_SIZE);
    c.text("SwiftTray",Rect::new(20,161,WIDTH-40,34),c.theme.text,5,DT_CENTER);
    c.text(&format!("Version {}",env!("CARGO_PKG_VERSION")),
        Rect::new(24,199,WIDTH-48,22),c.theme.dim,1,DT_CENTER);
    c.brand_gradient_box(Rect::new(142,229,76,3),2);
    c.text("Fast audio control for Windows 11",
        Rect::new(18,244,WIDTH-36,26),c.theme.text,0,DT_CENTER);

    // Text + icon only. No button surface, border, or background on hover.
    let check=Rect::new(49,272,WIDTH-98,37);
    let highlight=app.hover==Some(Action::CheckUpdates);
    let checking=matches!(&app.update_status,UpdateStatus::Checking);
    let label=if checking {"Checking for updates..."} else {"Check for updates"};
    let label_width=c.measure_small_text(label).clamp(95,223);
    let total_width=label_width+30;
    let x=(WIDTH-total_width)/2;
    let on_hover=if c.theme.brand {BRAND_HOVER} else {lighten_color(c.theme.accent,0.30)};
    let action_color=if highlight {on_hover}else{c.theme.accent};
    c.symbol("\u{E895}",Rect::new(x,279,23,23),action_color);
    c.text(label,Rect::new(x+30,276,label_width+8,28),
        if highlight {on_hover}else{c.theme.text},0,0);
    app.add_hit(check,Action::CheckUpdates);

    // No error is shown before the user initiates a check. Short status text
    // never overruns the flyout, and missing configuration is not an alert.
    let status_rect=Rect::new(20,310,WIDTH-40,25);
    match &app.update_status {
        UpdateStatus::Unconfigured=>c.text("Update source not configured",
            status_rect,c.theme.dim,1,DT_CENTER),
        UpdateStatus::Current(version)=>c.text(&format!("You're up to date (v{version})"),
            status_rect,c.theme.accent,1,DT_CENTER),
        UpdateStatus::Available{version,..}=>c.text(&format!("Update {version} available - View release"),
            status_rect,c.theme.accent,1,DT_CENTER),
        UpdateStatus::Error(message)=>c.text(friendly_update_error(message),
            status_rect,c.theme.dim,1,DT_CENTER),
        _=>{},
    }
    if matches!(&app.update_status,UpdateStatus::Available{..}) {
        app.add_hit(Rect::new(20,308,WIDTH-40,29),Action::OpenUpdateRelease);
    }

    c.line(22,ABOUT_FOOTER_RULE,WIDTH-44,c.theme.divider);
    // The repository is a true link only in configured builds. Never open an
    // invented placeholder or present a dead row as a functional button.
    let repository_active=updates::repository_url().is_some();
    let source_hover=app.hover==Some(Action::OpenRepository);
    let source_color=if !repository_active {c.theme.dim}
        else if source_hover {on_hover} else {c.theme.accent};
    c.symbol("\u{E8A7}",Rect::new(36,355,22,24),source_color);
    c.text("Open source",Rect::new(65,355,109,24),
        if !repository_active {c.theme.dim}
        else if source_hover {on_hover}else{c.theme.text},0,0);
    if repository_active {app.add_hit(Rect::new(25,348,154,38),Action::OpenRepository);}
    c.box_color(Rect::new(184,358,1,18),c.theme.divider,0);
    let license_hover=app.hover==Some(Action::OpenLicense);
    c.symbol("\u{E8A5}",Rect::new(201,355,22,24),
        if license_hover {on_hover}else{c.theme.accent});
    c.text("MIT License",Rect::new(231,355,108,24),
        if license_hover {on_hover}else{c.theme.text},0,0);
    app.add_hit(Rect::new(195,348,148,38),Action::OpenLicense);
}
fn draw_welcome(app:&mut App,c:&Canvas,_a:u32) {
    // A restrained brand-tinted header: never force Windows' accent to match the icon.
    c.app_logo(if app.about_icon!=0 {app.about_icon}else{app.tray_icon},
        if app.about_image!=0 {app.about_image}else{app.tray_image},29,24,39);
    c.text("Welcome to SwiftTray",Rect::new(88,23,WIDTH-106,33),c.theme.text,5,0);
    c.line(23,79,WIDTH-46,c.theme.divider);
    c.text("Launch at startup?",Rect::new(24,90,WIDTH-48,28),c.theme.text,2,0);
    c.text("Start SwiftTray automatically when you sign in.",
        Rect::new(24,121,WIDTH-48,23),c.theme.dim,1,0);
    c.text("You can change this later in Settings.",
        Rect::new(24,145,WIDTH-48,22),c.theme.dim,1,0);
    let skip=Rect::new(24,188,146,39);
    let enable=Rect::new(180,188,155,39);
    c.box_color(skip,if app.hover==Some(Action::StartupSkip){c.theme.panel}else{c.theme.rail},9);
    c.box_color(enable,if app.hover==Some(Action::StartupEnable){lighten_color(c.theme.accent,0.22)}else{c.theme.accent},9);
    c.text("Not now",skip,c.theme.text,2,DT_CENTER);
    c.text("Enable",enable,BRAND_INK,2,DT_CENTER);
    app.add_hit(skip,Action::StartupSkip);
    app.add_hit(enable,Action::StartupEnable);
}

fn draw_tray_menu(app:&mut App,c:&Canvas) {
    // Compact, consistent menu using the app's original icon and the same
    // 23-DIP Segoe Fluent symbol footprint for each action.
    c.app_logo(app.tray_menu_icon,app.tray_image,18,12,25);
    c.text("SwiftTray",Rect::new(54,11,163,29),c.theme.text,2,0);
    c.line(17,46,196,c.theme.divider);
    let choices=[("Open mixer","\u{E767}",Action::TrayOpen),
        ("Settings","\u{E713}",Action::TraySettings),
        ("Check for updates","\u{E895}",Action::TrayCheckUpdates),
        ("About","\u{E946}",Action::TrayAbout),
        ("Exit","\u{E7E8}",Action::TrayExit)];
    for (i,(label,glyph,action)) in choices.into_iter().enumerate() {
        let y=52+i as i32*38 + if i==4 {TRAY_EXIT_OFFSET} else {0};
        if i==4 {c.line(17,y-7,196,c.theme.divider);}
        let hover=app.hover==Some(action) || (app.hover.is_none() && app.tray_choice==i);
        let row=Rect::new(10,y,210,34);
        if hover {
            c.alpha_box(row,c.theme.text,13,8);
            c.box_color(Rect::new(10,y+7,2,20),c.theme.accent,1);
        }
        let icon_color=if i==4 {if hover {c.theme.text}else{c.theme.dim}}
            else if hover {c.theme.accent}else{c.theme.dim};
        c.symbol(glyph,Rect::new(20,y+4,23,25),icon_color);
        c.text(label,Rect::new(55,y+4,158,26),c.theme.text,0,0);
        app.add_hit(row,action);
    }
}
fn paint(app:&mut App,hwnd:H,toast:bool) {
    let mut ps:PaintStruct=unsafe {mem::zeroed()};
    let target=unsafe {BeginPaint(hwnd,&mut ps)};
    if target==0 {return;}
    unsafe {
        let mut bounds=Rect::default();GetClientRect(hwnd,&mut bounds);
        let memory=CreateCompatibleDC(target);
        let bitmap=CreateCompatibleBitmap(target,bounds.width(),bounds.height());
        if bitmap!=0&&memory!=0 {
            let old=SelectObject(memory,bitmap);
            // Initialize the memory DC on every frame so rounded corners do
            // not contain random pixels after a resize or DPI transition.
            PatBlt(memory,0,0,bounds.width(),bounds.height(),BLACKNESS);
            let canvas=Canvas::new(memory,app.dpi,app.palette);
            if toast {
                // A compact copy of the mixer header: same surface, output
                // notch, Fluent device symbol, accent and typography. The
                // two-line layout keeps the notification readable at 248 DIP.
                draw_surface(&canvas,248,62);
                output_notch(&canvas,14,20);
                canvas.large_symbol(&app.toast_symbol,Rect::new(24,14,29,30),canvas.theme.accent);
                canvas.text("AUDIO OUTPUT",Rect::new(64,7,169,19),canvas.theme.dim,1,0);
                canvas.text(&app.toast_text,Rect::new(64,28,166,26),canvas.theme.text,5,0);
            } else {draw_app(app,&canvas);}
            drop(canvas);
            BitBlt(target,0,0,bounds.width(),bounds.height(),memory,0,0,SRCCOPY);
            SelectObject(memory,old);DeleteObject(bitmap);DeleteDC(memory);
        } else if memory!=0 {DeleteDC(memory);}
        EndPaint(hwnd,&ps);
    }
}

unsafe extern "system" fn window_proc(hwnd:H,msg:u32,w:usize,l:isize)->isize {
    if APP.is_null() {return DefWindowProcW(hwnd,msg,w,l);}
    let app=&mut *APP;
    if msg==app.taskbar_created && app.taskbar_created!=0 {add_tray(app);return 0;}
    if hwnd==app.toast {
        match msg {
            WM_PAINT=>{paint(app,hwnd,true);return 0;},
            WM_TIMER if w==TIMER_TOAST=>{KillTimer(hwnd,TIMER_TOAST);ShowWindow(hwnd,SW_HIDE);return 0;},
            _=>return DefWindowProcW(hwnd,msg,w,l),
        }
    }
    match msg {
        WM_PAINT=>{paint(app,hwnd,false);return 0;},
        WM_CTLCOLOREDIT if l==app.edit => {
            SetBkColor(w as H,app.palette.panel);
            SetTextColor(w as H,app.palette.text);
            return app.edit_brush;
        },
        MSG_CAPTURE_KEYS_RELEASED=>{
            capture::disarm();
            if let Err(error)=capture::start_screen_snip() {
                app.capture_session.finish(app.window);
                app.capture_error(error);
            }
            return 0;
        },
        WM_HOTKEY=>{
            if app.capture_shortcut.is_some() {return 0;}
            match w as i32 {
                HOTKEY_POPUP=>app.toggle(),HOTKEY_SWITCH=>app.next_device(),HOTKEY_SNIP=>app.snip(),_=>{}
            }return 0;
        },
        MSG_F12_SHORTCUT=>{
            if app.capture_shortcut.is_some() {return 0;}
            match hotkeys::f12_action() {
                0=>app.toggle(),1=>app.next_device(),2=>app.snip(),_=>{},
            }
            return 0;
        },
        WM_TIMER if w==TIMER_SLIDER=>{app.flush_slider();return 0;},
        WM_TIMER if w==TIMER_HOTKEY_RETRY=>{app.retry_unavailable_hotkeys();return 0;},
        WM_TIMER if w==TIMER_UPDATE_RESULT=>{app.receive_update_result();return 0;},
        WM_TIMER if w==TIMER_METER=>{
            if app.visible && app.page==Page::Mixer {
                if let Some(audio)=app.audio.as_mut() {
                    if audio.sample_peaks() { app.repaint(); }
                }
            }
            return 0;
        },
        WM_TIMER if w==TIMER_APP_TOOLTIP=>{
            KillTimer(hwnd,TIMER_APP_TOOLTIP);
            if app.hover_is_audio_icon() && app.dragging.is_none() {
                app.tooltip_visible=true;
                app.repaint();
            }
            return 0;
        },
        MSG_CAPTURE_FOREGROUND=>{
            if app.capture_session.on_foreground(app.window,w as H) {
                // The capture overlay has yielded focus. Normal focus policy
                // resumes now, after the capture has finished or been cancelled.
                if app.visible && GetForegroundWindow()!=app.window { app.hide(); }
            }
            return 0;
        },
        WM_CLIPBOARDUPDATE=>{
            if app.capture_session.active() {
                app.capture_session.finish(app.window);
                // Clipboard notification is emitted after a successful snip.
                if app.visible && GetForegroundWindow()!=app.window { app.hide(); }
            }
            return 0;
        },
        WM_DEVICECHANGE=>{
            // USB/Bluetooth arrival and removal may change the list of active
            // endpoints. Refresh without having to close and reopen the flyout.
            // The hidden flyout refreshes the list next time it is opened.
            if app.visible && app.dragging.is_none() && app.edit_device.is_none() {
                let before=app.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                if let Some(audio)=app.audio.as_mut() {let _=audio.refresh();}
                else {app.audio=Audio::new().ok();}
                let after=app.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                if after!=before {
                    app.mixer_rows_floor=app.mixer_rows_floor.max(after);
                    app.page_offset=0;
                    app.resize();
                }
                app.repaint();
            }
            return 0;
        },
        WM_DISPLAYCHANGE=>{
            if app.visible {app.resize();app.repaint();}
            return 0;
        },
        WM_DPICHANGED=>{
            // Use the OS-recommended rectangle first so MonitorFromWindow sees
            // the destination display, then recalculate our compact flyout size
            // in 96-DIP coordinates at the destination DPI.
            let suggested=l as *const Rect;
            if !suggested.is_null() {
                let rect=&*suggested;
                SetWindowPos(hwnd,0,rect.left,rect.top,rect.width(),rect.height(),
                    SWP_NOZORDER|SWP_NOACTIVATE);
            }
            app.dpi=((w & 0xFFFF) as u32).max(96);
            let font=CreateFontW(-app.s(14),0,0,0,400,0,0,0,1,0,0,5,0,wide("Segoe UI").as_ptr());
            if font!=0 {
                if app.edit!=0 {SendMessageW(app.edit,WM_SETFONT,font as usize,1);}
                if app.edit_font!=0 {DeleteObject(app.edit_font);}
                app.edit_font=font;
            }
            app.resize();app.repaint();
            return 0;
        },
        WM_TIMER if w==TIMER_REFRESH=>{
            let palette=Palette::from_config(&app.config);
            if palette.accent!=app.palette.accent || palette.background!=app.palette.background {
                app.palette=palette;
            }
            if app.visible && app.page==Page::Mixer && app.dragging.is_none() && app.edit_device.is_none() && app.capture_shortcut.is_none() {
                let previous=app.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                if let Some(audio)=app.audio.as_mut() {let _=audio.refresh();}
                else {app.audio=Audio::new().ok();}
                let current=app.audio.as_ref().map(|a|a.sessions.len()).unwrap_or(0);
                if previous!=current {
                    app.mixer_rows_floor=app.mixer_rows_floor.max(current);
                    app.page_offset=0;app.resize();
                }
                app.repaint();
            }
            return 0;
        },
        WM_SETTINGCHANGE|WM_THEMECHANGED|WM_SYSCOLORCHANGE|WM_DWMCOLORIZATIONCOLORCHANGED=>{
            app.palette=Palette::from_config(&app.config);
            if app.edit_brush!=0 { DeleteObject(app.edit_brush); }
            app.edit_brush=CreateSolidBrush(app.palette.panel);
            let dark:u32=if background_is_dark(app.palette.background) {1}else{0};
            for handle in [app.window,app.toast] {
                DwmSetWindowAttribute(handle,20,&dark as *const _ as *const c_void,4);
            }
            let _=backdrop::configure(app.window,dark!=0);
            app.repaint();return 0;
        },
        WM_ACTIVATE=>{
            // Clicking away closes the flyout, like Windows Quick Settings.
            if (w & 0xffff)==0 && app.visible && app.edit_device.is_none()
                && !app.capture_session.suppress_focus_close() {
                app.hide();
            }
            return 0;
        },
        WM_LBUTTONDOWN=>{
            app.tooltip_visible=false;
            KillTimer(hwnd,TIMER_APP_TOOLTIP);
            let x=x_of(l)*96/app.dpi as i32;
            let y=y_of(l)*96/app.dpi as i32;
            let hit=app.hits.iter().rev().find(|h|h.rect.contains(x,y)).cloned();
            if let Some(h)=hit {
                let slider=matches!(h.action,Action::MasterSlider|Action::AppSlider(_));
                app.action(h.action,x,y);
                if slider { app.update_hover(Some(h.action)); SetCapture(hwnd); }
            }
            return 0;
        },
        WM_MOUSEMOVE=>{
            if !app.tracking_mouse {
                let mut tracking=TrackMouseEventData {
                    size:mem::size_of::<TrackMouseEventData>() as u32,
                    flags:TME_LEAVE,window:hwnd,hover_time:0,
                };
                app.tracking_mouse=TrackMouseEvent(&mut tracking)!=0;
            }
            let x=x_of(l)*96/app.dpi as i32;
            let y=y_of(l)*96/app.dpi as i32;
            if let Some(slider)=app.dragging {
                app.drag_slider(slider,x);
            } else {
                let hover=app.hits.iter().rev().find(|h|h.rect.contains(x,y)).map(|h|h.action);
                app.update_hover(hover);
            }
            return 0;
        },
        WM_MOUSELEAVE=>{
            app.tracking_mouse=false;
            app.update_hover(None);
            return 0;
        },
        WM_LBUTTONUP=>{
            if app.dragging.take().is_some(){
                app.flush_slider();
                ReleaseCapture();
                app.repaint(); // Remove the percentage bubble immediately.
            }
            return 0;
        },
        WM_CAPTURECHANGED=>{
            app.dragging=None;
            app.flush_slider();
            app.repaint();
            return 0;
        },
        WM_MOUSEWHEEL=>{
            let mut pt=Point{x:x_of(l),y:y_of(l)};
            ScreenToClient(hwnd,&mut pt);
            let x=pt.x*96/app.dpi as i32;
            let y=pt.y*96/app.dpi as i32;
            let delta=((w>>16) as u16 as i16) as f32/120.0;
            let target=app.hits.iter().rev().find(|h|h.rect.contains(x,y))
                .map(|h|h.action);
            if let Some(action @ (Action::MasterSlider|Action::AppSlider(_)))=target {
                let level=match action {
                    Action::MasterSlider=>app.audio.as_ref().map(|a|a.master_level).unwrap_or(0.0),
                    Action::AppSlider(i)=>app.audio.as_ref().and_then(|a|a.sessions.get(i))
                        .map(|s|s.level).unwrap_or(0.0),_=>0.0,
                };
                let next=(level+delta*0.05).clamp(0.0,1.0);
                let new_x=SLIDER_LEFT+(next*(SLIDER_RIGHT-SLIDER_LEFT) as f32) as i32;
                app.drag_slider(action,new_x);app.flush_slider();
            }
            return 0;
        },
        WM_KEYDOWN|WM_SYSKEYDOWN=>{
            if let Some(i)=app.capture_shortcut {
                if w as u32==VK_ESCAPE {app.cancel_capture();app.repaint();return 0;}
                let key=w as u32;
                if [0x10,0x11,0x12,0x5B,0x5C].contains(&key) {return 0;}
                let mut mods=0;
                if GetKeyState(VK_CONTROL)<0 {mods|=0x0002;}
                if GetKeyState(VK_MENU)<0 {mods|=0x0001;}
                if GetKeyState(VK_SHIFT)<0 {mods|=0x0004;}
                if GetKeyState(VK_LWIN)<0||GetKeyState(VK_RWIN)<0 {mods|=0x0008;}
                app.capture_shortcut=None;
                app.assign_hotkey(i,Shortcut::new(mods,key));
                app.repaint();
                return 0;
            }
            if app.page==Page::TrayMenu {
                match w as u32 {
                    0x26=>{app.tray_choice=(app.tray_choice+4)%5;app.hover=None;app.repaint();return 0;},
                    0x28=>{app.tray_choice=(app.tray_choice+1)%5;app.hover=None;app.repaint();return 0;},
                    0x0D=>{
                        let choice=match app.tray_choice {0=>Action::TrayOpen,1=>Action::TraySettings,2=>Action::TrayCheckUpdates,3=>Action::TrayAbout,_=>Action::TrayExit};
                        app.action(choice,0,0);return 0;
                    },
                    _=>{},
                }
            }
            if w as u32==VK_ESCAPE {app.hide();return 0;}
        },
        TRAY_EVENT=>{
            match (l as u32)&0xffff {
                0x0202|0x0203|0x0400=>app.show(), // A tray click opens; a second click must not hide the flyout.
                0x0205|0x007B=>app.show_tray_menu(),  // custom accent-aware context flyout
                _=>{}
            }
            return 0;
        },
        WM_COMMAND=>{
            if (w&0xffff)==EDIT_ID && ((w>>16)&0xffff) as u32==EN_KILLFOCUS {
                // Keep current typed name until user taps Save explicitly.
            }
        },
        WM_CLOSE=>{app.hide();return 0;},
        WM_DESTROY=>{
            KillTimer(hwnd,TIMER_HOTKEY_RETRY);
            KillTimer(hwnd,TIMER_UPDATE_RESULT);
            capture::disarm();
            remove_tray(app);
            app.capture_session.finish(hwnd);
            for i in 0..3 {app.unregister_binding(i);}
            PostQuitMessage(0);return 0;
        },
        _=>{}
    }
    DefWindowProcW(hwnd,msg,w,l)
}

// Native logical 16px taskbar icon, independently rasterized for each common
// display scale. The 32px frame is the largest system tray variant.
fn tray_icon_pixels(dpi:u32)->i32 {
    ((16 * dpi.clamp(96,192) + 48) / 96).clamp(16,32) as i32
}

// Version 4 suppresses the standard Shell tooltip unless NIF_SHOWTIP is set.
// Use the flag on NIM_ADD, including after an Explorer/taskbar restart.
fn tray_notification_flags()->u32 {
    NIF_ICON|NIF_MESSAGE|NIF_TIP|NIF_SHOWTIP
}

unsafe fn notify_data(app:&App)->NotifyIconData {
    let mut data:NotifyIconData=mem::zeroed();
    data.size=mem::size_of::<NotifyIconData>() as u32;
    data.window=app.window;data.id=1;
    data.flags=tray_notification_flags();
    data.icon=app.tray_icon;data.callback=TRAY_EVENT;
    copy_wide(&mut data.tip,TRAY_TOOLTIP);
    data
}
unsafe fn add_tray(app:&App) {
    let mut data=notify_data(app);
    Shell_NotifyIconW(NIM_ADD,&mut data);
    data.timeout_or_version=NOTIFYICON_VERSION_4;
    Shell_NotifyIconW(NIM_SETVERSION,&mut data);
}
unsafe fn remove_tray(app:&App) {
    let mut data=notify_data(app);Shell_NotifyIconW(NIM_DELETE,&mut data);
}

/// Initializes the app on the Windows UI thread and enters the native message loop.
pub fn run() -> Result<(), String> {
    // Single-instance guard: duplicate processes cannot steal each other's hotkeys.
    let mutex_name=wide("Local\\SwiftTrayNativeSingleInstance");
    let mutex=unsafe {crate::singleton::CreateMutexW(ptr::null(),0,mutex_name.as_ptr())};
    if mutex!=0 && unsafe {crate::singleton::GetLastError()}==183 {
        unsafe {close_mutex(mutex);}
        return Err("SwiftTray is already running in the system tray.".into());
    }
    unsafe {SetProcessDpiAwarenessContext(-4); /* PER_MONITOR_AWARE_V2 */}
    let init=unsafe {CoInitializeEx(None,COINIT_APARTMENTTHREADED)};
    if init.is_err() {return Err("COM initialization failed".into());}
    // WinRT UISettings needs a Windows Runtime apartment for reliable accent
    // and system theme detection. RoInitialize and CoInitializeEx share STA.
    let winrt=unsafe {RoInitialize(0)}>=0;
    let smooth_token=smooth::start().unwrap_or(0);
    let audio=Audio::new().ok();
    let instance=unsafe {GetModuleHandleW(ptr::null())};
    let class_name=wide("SwiftTrayFlyoutClass");
    let toast_class_name=wide("SwiftTrayToastClass");
    // Resource ID 1 is the user's native multi-size ICO, embedded in the EXE.
    let embedded_icon=unsafe {LoadIconW(instance,1usize as *const u16)};
    let icon=if embedded_icon!=0 {embedded_icon}else{unsafe {LoadIconW(0,SYSTEM_ICON)}};
    let cursor=unsafe {LoadCursorW(0,ARROW_CURSOR)};
    let make_class=|name:*const u16|WindowClass {
        size:mem::size_of::<WindowClass>() as u32, style:0x00020000, // CS_DROPSHADOW
        callback:Some(window_proc),extra_class:0,extra_window:0,
        instance,icon,cursor,background:0,menu_name:ptr::null(),class_name:name,small_icon:icon,
    };
    unsafe {
        if RegisterClassExW(&make_class(class_name.as_ptr()))==0||
            RegisterClassExW(&make_class(toast_class_name.as_ptr()))==0 {
            smooth::stop(smooth_token);
            if winrt { RoUninitialize(); }
            CoUninitialize();close_mutex(mutex);
            return Err("Could not register Windows flyout class".into());
        }
    }
    let mut app=Box::new(App::new(Config::load(),audio));
    // IMAGE_ICON=1; request the real taskbar size so that our tuned 16/20/24px
    // ICO frames are used at 100/125/150% scale instead of enlarging 16px.
    let tray_size=tray_icon_pixels(unsafe {GetDpiForSystem()});
    let small=unsafe {LoadImageW(instance,1usize as *const u16,1,tray_size,tray_size,0)};
    app.tray_owned=small!=0;
    app.tray_icon=if small!=0 {small}else{icon};
    // The 230-DIP context menu needs a 32px source, not an enlarged 16px HICON.
    let menu_icon=unsafe {LoadImageW(instance,1usize as *const u16,1,32,32,0)};
    app.tray_menu_icon=if menu_icon!=0 {menu_icon}else{app.tray_icon};
    // About uses a dedicated 256px PArgb raster embedded directly from the
    // original SVG, not GdipCreateBitmapFromHICON (which distorted its edges).
    // Keep a native HICON as a fallback if GDI+ cannot create the bitmap.
    app.about_icon=unsafe {LoadImageW(instance,1usize as *const u16,1,128,128,0)};
    app.tray_image=smooth::prepare_native_icon(app.tray_menu_icon);
    app.about_pixels=smooth::about_logo_pixels();
    app.about_image=smooth::prepare_about_logo(&mut app.about_pixels);
    if app.about_image==0 && app.about_icon!=0 {
        app.about_image=smooth::prepare_native_icon(app.about_icon);
    }
    let window=unsafe {
        CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class_name.as_ptr(),wide("SwiftTray").as_ptr(),WS_POPUP,
            0,0,WIDTH,400,0,0,instance,ptr::null_mut())
    };
    let toast=unsafe {
        CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_NOACTIVATE,
            toast_class_name.as_ptr(),wide("SwiftTray Toast").as_ptr(),WS_POPUP,
            0,0,248,62,0,0,instance,ptr::null_mut())
    };
    if window==0||toast==0 {
        unsafe {smooth::stop(smooth_token);if winrt {RoUninitialize();} CoUninitialize();close_mutex(mutex);}
        return Err("Could not create native Windows window".into());
    }
    let edit=unsafe {CreateWindowExW(0,wide("EDIT").as_ptr(),ptr::null(),
        WS_CHILD|ES_AUTOHSCROLL|WS_TABSTOP,0,0,120,29,
        window,EDIT_ID as H,instance,ptr::null_mut())};
    app.window=window;app.toast=toast;app.edit=edit;
    app.edit_brush=unsafe {CreateSolidBrush(app.palette.panel)};
    app.dpi=unsafe {GetDpiForWindow(window).max(96)};
    app.edit_font=unsafe {CreateFontW(-app.s(14),0,0,0,400,0,0,0,1,0,0,5,0,wide("Segoe UI").as_ptr())};
    unsafe {SendMessageW(edit,WM_SETFONT,app.edit_font as usize,1);}
    app.taskbar_created=unsafe {RegisterWindowMessageW(wide("TaskbarCreated").as_ptr())};
    let corner: u32=2; // DWMWCP_ROUND.
    let dark:u32=if background_is_dark(app.palette.background) {1}else{0}; // DWMWA_USE_IMMERSIVE_DARK_MODE.
    unsafe {
        for handle in [window,toast] {
            DwmSetWindowAttribute(handle,33,&corner as *const _ as *const c_void,4);
            DwmSetWindowAttribute(handle,20,&dark as *const _ as *const c_void,4);
        }
    }
    let _=backdrop::configure(window,dark!=0);
    let app_ptr=Box::into_raw(app);
    unsafe {
        APP=app_ptr;
        (*APP).hotkeys();
        add_tray(&*APP);
        // One-time, opt-in prompt. No startup registry entry is created without consent.
        if !(*APP).config.startup_prompt_seen { (*APP).show_welcome(); }
        let mut msg:Msg=mem::zeroed();
        while GetMessageW(&mut msg,0,0,0)>0 {TranslateMessage(&msg);DispatchMessageW(&msg);}
        smooth::release_native_icon((*APP).tray_image);
        smooth::release_native_icon((*APP).about_image);
        if (*APP).tray_menu_icon!=0 && (*APP).tray_menu_icon!=(*APP).tray_icon {
            DestroyIcon((*APP).tray_menu_icon);
        }
        if (*APP).tray_owned { DestroyIcon((*APP).tray_icon); }
        if (*APP).about_icon!=0 { DestroyIcon((*APP).about_icon); }
        DeleteObject((*APP).edit_font);
        DeleteObject((*APP).edit_brush);
        let owned=Box::from_raw(APP); APP=ptr::null_mut();drop(owned);
        smooth::stop(smooth_token);
        if winrt {RoUninitialize();}
        CoUninitialize();close_mutex(mutex);
    }
    Ok(())
}

unsafe fn close_mutex(handle:H) {
    if handle!=0 {crate::singleton::CloseHandle(handle);}
}

#[cfg(test)]
mod compact_header_tests {
    use super::{MIXER_FIRST,ROW_HEIGHT};

    #[test]
    fn first_slider_moves_with_the_heading() {
        assert_eq!(MIXER_FIRST,94);
        assert_eq!(MIXER_FIRST+ROW_HEIGHT,140);
    }
}

#[cfg(test)]
mod visual_geometry_tests {
    use super::*;
    #[test]
    fn volume_bubble_is_centered_and_in_bounds_at_0_and_100() {
        for thumb_x in [SLIDER_LEFT,SLIDER_RIGHT] {
            let popup=value_bubble_rect(thumb_x,125);
            assert_eq!(popup.left+popup.width()/2,thumb_x);
            assert!(popup.left>=0 && popup.right<=WIDTH);
        }
    }
    #[test]
    fn app_hint_does_not_cover_its_slider_or_the_next_icon() {
        for row in [MIXER_FIRST,MIXER_FIRST+ROW_HEIGHT,MIXER_FIRST+ROW_HEIGHT*3] {
            let tip=tooltip_rect(row,148);
            assert!(tip.top>row+24); // below the hovered row's track
            assert!(tip.bottom<row+ROW_HEIGHT+10); // before next row icon
            assert!(tip.right<=WIDTH);
        }
    }
    #[test]
    fn known_app_labels_are_readable_without_rebranding_the_icon() {
        assert_eq!(clean_audio_label("brave"),"brave");
        assert_eq!(clean_audio_label("System sounds"),"System sounds");
    }
}

#[cfg(test)]
mod visual_meter_tests {
    use super::{activity_length, brand_color_at, mix_color, slider_progress,
        BRAND_GREEN, BRAND_TEAL, BRAND_BLUE, BRAND_DEEP_BLUE};

    #[test]
    fn slider_and_activity_lengths_use_their_normalized_inputs() {
        assert_eq!(slider_progress(0.7, 200), 140);
        assert_eq!(activity_length(0.9, 200), 180);
        assert_eq!(slider_progress(1.0, 200), 200);
        assert_eq!(activity_length(-1.0, 200), 0);
    }

    #[test]
    fn brand_gradient_has_fixed_stops() {
        assert_eq!(brand_color_at(0.0), BRAND_GREEN);
        assert_eq!(brand_color_at(0.53), BRAND_TEAL);
        assert_eq!(brand_color_at(0.86), BRAND_BLUE);
        assert_eq!(brand_color_at(1.0), BRAND_DEEP_BLUE);
        assert_eq!(mix_color(BRAND_GREEN, BRAND_TEAL, 0.0), BRAND_GREEN);
        assert_eq!(mix_color(BRAND_BLUE, BRAND_DEEP_BLUE, 1.0), BRAND_DEEP_BLUE);
    }
}

#[cfg(test)]
mod v017_layout_tests {
    use super::{activity_lane_y,slider_progress};

    #[test]
    fn activity_lane_is_two_dip_lower_than_previous_release() {
        let thumb=150;
        let volume_track_bottom=thumb+3;
        assert_eq!(activity_lane_y(thumb),thumb+10);
        assert_eq!(activity_lane_y(thumb)-volume_track_bottom,7);
    }

    #[test]
    fn silent_activity_does_not_create_a_second_track() {
        // Activity is rendered only if the measured peak is meaningful.
        assert!(super::activity_length(0.0,200)<3);
        assert!(super::activity_length(0.01,200)<3);
        assert_eq!(super::activity_length(0.5,200),100);
    }

    #[test]
    fn current_drag_position_caps_the_live_meter() {
        let length=260;
        let peak=0.73;
        for volume in [0.0,0.03,0.12,0.5,0.9,1.0] {
            let thumb=slider_progress(volume,length);
            let displayed=super::displayed_activity_peak(peak,volume,false);
            assert!(slider_progress(displayed,length)<=thumb);
        }
        assert_eq!(super::displayed_activity_peak(peak,1.0,true),0.0);
    }
}

#[cfg(test)]
mod about_and_tray_layout_tests {
    use super::{friendly_update_error, ABOUT_FOOTER_RULE, ABOUT_LOGO_SIZE,
        ABOUT_LOGO_TOP, TRAY_EXIT_OFFSET};

    #[test]
    fn about_logo_stays_above_version_and_footer() {
        assert!(ABOUT_LOGO_TOP + ABOUT_LOGO_SIZE <= 161);
        assert!(ABOUT_FOOTER_RULE > 335);
        assert!(ABOUT_FOOTER_RULE + 44 <= 394);
    }
    #[test]
    fn exit_is_separated_without_overlapping_other_tray_rows() {
        let about_bottom=52 + 3*38 + 34;
        let exit_top=52 + 4*38 + TRAY_EXIT_OFFSET;
        assert!(exit_top > about_bottom);
        assert!(exit_top + 34 < 265);
    }
    #[test]
    fn update_errors_are_short_and_human_readable() {
        for msg in ["No published GitHub Release was found yet",
            "GitHub release check failed (HTTP 403)", "An unexpected network error"] {
            assert!(friendly_update_error(msg).len() <= 41);
        }
    }
}

#[cfg(test)]
mod shortcut_and_theme_tests {
    use super::{background_is_dark,tooltip_rect,WIDTH};
    use crate::native::rgb;

    #[test]
    fn dark_caption_detection_uses_brightness_not_magic_white_value() {
        assert!(background_is_dark(rgb(29,30,33)));
        assert!(!background_is_dark(rgb(255,255,255)));
    }

    #[test]
    fn compact_shortcut_rows_do_not_exceed_flyout_width() {
        let left=WIDTH-27-136;
        assert!(left>60);
        assert!(WIDTH-51-136>60);
        let _=tooltip_rect(80,100);
    }
}


#[cfg(test)]
mod tray_icon_scaling_tests {
    use super::tray_icon_pixels;
    #[test]
    fn native_taskbar_icon_sizes_match_common_dpi_scales() {
        assert_eq!(tray_icon_pixels(96),16);
        assert_eq!(tray_icon_pixels(120),20);
        assert_eq!(tray_icon_pixels(144),24);
        assert_eq!(tray_icon_pixels(192),32);
    }
    #[test]
    fn tiny_or_unexpected_dpi_never_selects_a_zero_size() {
        assert_eq!(tray_icon_pixels(0),16);
        assert_eq!(tray_icon_pixels(300),32);
    }
}

#[cfg(test)]
mod tray_tooltip_regression_tests {
    use super::{tray_notification_flags, TRAY_TOOLTIP};
    use crate::native::{copy_wide, NIF_TIP, NIF_SHOWTIP, NOTIFYICON_VERSION_4};

    #[test]
    fn version_four_requests_standard_shell_tooltip() {
        assert_eq!(NOTIFYICON_VERSION_4, 4);
        assert_ne!(tray_notification_flags() & NIF_TIP, 0);
        assert_ne!(tray_notification_flags() & NIF_SHOWTIP, 0);
    }

    #[test]
    fn tray_name_is_simple_and_null_terminated() {
        assert_eq!(TRAY_TOOLTIP, "SwiftTray");
        let mut field = [0u16; 128];
        copy_wide(&mut field, TRAY_TOOLTIP);
        let encoded: Vec<u16> = TRAY_TOOLTIP.encode_utf16().collect();
        assert_eq!(&field[..encoded.len()], &encoded);
        assert_eq!(field[encoded.len()], 0);
    }
}

#[cfg(test)]
mod visualizer_dash_tests {
    use super::{activity_lane_y,visualizer_bar_offsets};
    #[test]
    fn dashes_follow_the_processed_peak() {
        let low=visualizer_bar_offsets(0.25,270);
        let high=visualizer_bar_offsets(0.75,270);
        assert!(!low.is_empty());
        assert!(high.len()>low.len());
        assert!(visualizer_bar_offsets(0.0,270).is_empty());
    }
    #[test]
    fn dashes_never_extend_beyond_the_meter() {
        for p in [0.0,0.01,0.25,0.5,0.99,1.0] {
            let width=270;
            let active=(p*width as f32).round() as i32;
            for x in visualizer_bar_offsets(p,width) {assert!(x>=0 && x+3<=active);}
        }
    }
    #[test]
    fn visible_dashes_never_pass_the_volume_thumb() {
        for volume in [0.0,0.03,0.65,1.0] {
            let peak=super::displayed_activity_peak(0.90,volume,false);
            let thumb=super::slider_progress(volume,270);
            for x in visualizer_bar_offsets(peak,270) {
                assert!(x+3<=thumb);
            }
        }
    }
    #[test]
    fn activity_remains_in_its_own_lane() {
        assert_eq!(activity_lane_y(100),110);
    }
}
