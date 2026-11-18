//! A supervisor joins the kill-on-close job before it can launch yt-dlp.
//! Parent death before assignment also fails closed: opening the job fails, or
//! closing the supervisor's temporary handle kills it after assignment.
use crate::db::{err,Result};
use std::process::{Child,Command,Stdio};
#[cfg(windows)]
use std::{os::windows::{ffi::OsStrExt,io::{FromRawHandle,AsRawHandle,OwnedHandle},process::CommandExt},ffi::OsStr};
#[cfg(windows)]
use windows_sys::Win32::{System::{JobObjects::*,Threading::GetCurrentProcess}};

pub struct Tree {
 #[cfg(windows)]
 handle:OwnedHandle,
}
impl Tree {
 pub fn spawn(command:&Command)->Result<(Self,Child)> {
  #[cfg(windows)] {
   static SEQUENCE:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
   let name=format!("Local\\BitClever-download-{}-{}-{}",std::process::id(),crate::content::nonce(),SEQUENCE.fetch_add(1,std::sync::atomic::Ordering::Relaxed));
   let wide:Vec<u16>=OsStr::new(&name).encode_wide().chain(Some(0)).collect();
   // No inheritable handles: only the parent owns the job's lifetime.
   let handle=unsafe{CreateJobObjectW(std::ptr::null(),wide.as_ptr())};
   if handle.is_null(){return Err(err(std::io::Error::last_os_error()));}
   let owner=unsafe{OwnedHandle::from_raw_handle(handle)};
   let mut limits:JOBOBJECT_EXTENDED_LIMIT_INFORMATION=unsafe{std::mem::zeroed()};
   limits.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
   if unsafe{SetInformationJobObject(handle,JobObjectExtendedLimitInformation,&limits as *const _ as _,std::mem::size_of_val(&limits) as u32)}==0{return Err(err(std::io::Error::last_os_error()));}
   let mut helper=Command::new(std::env::current_exe().map_err(err)?);
   helper.creation_flags(0x08000000);
   #[cfg(not(test))]
   helper.arg("--design-download-worker");
   #[cfg(test)]
   helper.args(["--exact","download_process::tests::supervisor_worker","--ignored","--nocapture"]);
   #[cfg(not(test))]
   helper.arg(&name).arg(command.get_program()).args(command.get_args());
   #[cfg(test)] {
    let args:Vec<String>=std::iter::once(std::ffi::OsStr::new(&name)).chain(std::iter::once(command.get_program())).chain(command.get_args()).map(|s|s.to_string_lossy().into_owned()).collect();
    helper.env("DESIGN_SUPERVISOR_ARGS",serde_json::to_string(&args).map_err(err)?);
   }
   for (key,value) in command.get_envs(){if let Some(value)=value{helper.env(key,value);}else{helper.env_remove(key);}}
   if let Some(dir)=command.get_current_dir(){helper.current_dir(dir);}
   helper.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
   let child=helper.spawn().map_err(err)?;
   Ok((Self{handle:owner},child))
  }
  #[cfg(not(windows))] {
   let mut child=Command::new(command.get_program());child.args(command.get_args());
   for (key,value) in command.get_envs(){if let Some(value)=value{child.env(key,value);}else{child.env_remove(key);}}
   if let Some(dir)=command.get_current_dir(){child.current_dir(dir);}
   child.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
   Ok((Self{},child.spawn().map_err(err)?))
  }
 }
 pub fn terminate(&self){
  #[cfg(windows)] unsafe{TerminateJobObject(self.handle.as_raw_handle(),1);}
 }
}
impl Drop for Tree {fn drop(&mut self){self.terminate();}}
#[cfg(windows)]
fn supervise(args:Vec<std::ffi::OsString>)->Result<i32>{
 if args.len()<2{return Err("Missing supervised download arguments".into());}
 let wide:Vec<u16>=args[0].encode_wide().chain(Some(0)).collect();
 let handle=unsafe{OpenJobObjectW(0x0001 /* JOB_OBJECT_ASSIGN_PROCESS */,false as _,wide.as_ptr())};
 if handle.is_null(){return Err("The download owner is no longer running.".into());}
 let owner=unsafe{OwnedHandle::from_raw_handle(handle)};
 if unsafe{AssignProcessToJobObject(handle,GetCurrentProcess())}==0{return Err(err(std::io::Error::last_os_error()));}
 drop(owner);
 let mut command=Command::new(&args[1]);command.args(&args[2..]).stdin(Stdio::null()).creation_flags(0x08000000);
 Ok(command.status().map_err(err)?.code().unwrap_or(1))
}
pub fn worker_entry(){
 #[cfg(windows)] {
  let mut args=std::env::args_os().skip(1);
  if args.next().is_some_and(|s|s=="--design-download-worker"){
   let result=supervise(args.collect());if let Err(error)=&result{eprintln!("{error}");}
   std::process::exit(result.unwrap_or(1));
  }
 }
}
#[cfg(test)]
mod tests {
 #[cfg(windows)]
 #[test] #[ignore="Internal supervised process worker"]
 fn supervisor_worker(){
  let args:Vec<String>=serde_json::from_str(&std::env::var("DESIGN_SUPERVISOR_ARGS").unwrap()).unwrap();
  let args=args.into_iter().map(std::ffi::OsString::from).collect();
  std::process::exit(super::supervise(args).unwrap_or(1));
 }
}
