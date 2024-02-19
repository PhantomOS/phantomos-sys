//! Interfaces for managing and accessing devices in userspace
//!
//! Device operations belong to subsystem 2 (io subsystem)
//!

use core::ffi::{c_long, c_ulong, c_void};

use crate::{security::SecurityContext, uuid::Uuid};

use self::udev::DeviceCommandParameter;

#[cfg(doc)]
use crate::sys::io::{CHAR_RANDOMACCESS, CHAR_READABLE, CHAR_SEEKABLE, CHAR_WRITABLE};

use super::{
    fs::FileHandle,
    handle::{Handle, HandlePtr},
    io::IOHandle,
    isolation::NamespaceHandle,
    kstr::{KStrCPtr, KStrPtr},
    result::SysResult,
};

pub mod udev;

/// Configuration for a block device created by [`CreateBlockDevice`]
#[repr(C)]
pub struct BlockDeviceConfiguration {
    /// A user-friendly name for the block device
    pub label: KStrCPtr,
    /// A [`FileHandle`] that represents an access control list, which specifies the access permisions of the created device
    pub acl: HandlePtr<FileHandle>,
    /// Specifies the number of bytes which the device reports as "Optimistic", IE. performing I/O operations of this size is at least as efficient as performing I/O operations of any smaller size
    pub optimistic_io_size: c_ulong,
    /// Specifies the base of a [`CHAR_RANDOMACCESS`] `IOHandle` to expose
    ///
    /// If the handle does not have [`CHAR_RANDOMACCESS`], this must be set to `0`
    pub base: c_ulong,
    /// Specifies the extent (maximum size) of the `IOHandle` to expose
    pub extent: c_long,
}

/// Configuraton for a charater device reated by [`CreateCharDevice`]
#[repr(C)]
pub struct CharDeviceConfiguration {
    /// A user-friendly name for the character device
    pub label: KStrCPtr,
    /// A [`FileHandle`] that represents an access control list, which specifies the access permisions of the created device
    pub acl: HandlePtr<FileHandle>,
    /// Specifies the number of bytes which the device reports as "Optimistic", IE. performing I/O operations of this size is at least as efficient as performing I/O operations of any smaller size
    pub optimistic_io_size: u64,
}

/// A Handle to a device
#[repr(transparent)]
pub struct DeviceHandle(Handle);

/// Treats every object in the mounted filesystem as having the `default_acl`
///
/// This is default if the filesystem does not support ACLs or Legacy Permisions (such as FAT32)
pub const MOUNT_REPLACE_ACLS: u32 = 0x01;
/// Enables the use of InstallSecurityContext and legacy SUID/SGID bits on mounted objects.
/// Requires the MountPrivilagedExec kernel permission
pub const MOUNT_ALLOW_PRIVILAGED: u32 = 0x02;
/// Treats every object in the mounted filesystem as having `default_acl` if the filesystem only supports legacy permissions
///
/// Note that some filesystems may support a form of ACL, but be considered to only support legacy permissions (for example, ext4's posix acl support).
/// This flag will override the ACLs on objects on which such ACLs are present
pub const MOUNT_REPLACE_LEGACY_PERMISSIONS: u32 = 0x04;

/// Specifies options for [`MountFilesystem`]

#[repr(C)]
pub struct MountOptions {
    /// The default ACL to use if the filesystem does not support permissions or where replacement is required
    pub default_acl: HandlePtr<FileHandle>,
    /// flags for the mount operation
    pub flags: u32,
    /// If the filesystem uses legacy permissions (or supports only posix acls, rather than enhanced dacls), then use this principal map given to map to Lilium principals.
    /// The IOHandle must have `CHAR_READ` and `CHAR_SEEK`. If it does not have `CHAR_RANDOMACCESS` then the behaviour is undefined if the thread calls `IOSeek`, or performs an I/O operation on the handle.
    pub legacy_principal_map: HandlePtr<IOHandle>,
}

#[allow(improper_ctypes)]
extern "C" {

    /// Creates a new block device backed by `backing_hdl`, with the specified configuration.
    ///
    /// `backing_hdl` must be seekable [`CHAR_SEEKABLE`]. If it is random-access [`CHAR_RANDOMACCESS`] that will be exposed to handles opened to it.
    ///
    /// The readability and writability of the device are those of the `IOHandle`,
    ///  but are limited by the permissions of the process that opens the device and the `acl` specified in `cfg`.
    ///
    /// `id` can be assigned by either the process or the kernel. If `id` is set to the nil UUID (all zeroes), the kernel will generate a device id and store it in `id`.
    ///  Otherwise, the device is assigned the `id` specified in `id` if it is unused.
    ///
    /// If `ns` is specified, then the device is created inside that namespace only. Otherwise, the device is created in the device scope of the current thread.
    ///
    /// ## Errors
    ///
    /// If `backing_hdl` is not a valid `IOHandle`, returns `INVALID_HANDLE`.
    ///
    /// If `backing_hdl` was previouslly passed to either this function or [`CreateCharDevice`] and the created device has not been remoed, returns `DEVICE_UNAVAILABLE`.
    ///
    /// If `backing_hdl` does not have [`CHAR_SEEKABLE`], returns `INVALID_OPERATION`. If `backing_hdl` does not have [`CHAR_RANDOMACCESS`] and `cfg.base` is not set to `0`, returns `INVALID_OPERATION`.
    ///
    /// If the current thread does not have the kernel permission `CREATE_BLOCK_DEVICE`, returns `PERMISSION`.
    ///
    /// If `id` is set to an explicit ID, `ns` is not an explicit namespace handle that has devices isolated, and the current thread does not have `ASSIGN_DEVICE_ID` permission,
    ///  returns `PERMISSION`.
    ///
    /// If `id` is set to an explicit ID, and that id is already in use by a device within the device scope of the specified namespace, returns `ALREADY_EXISTS`.
    ///
    pub fn CreateBlockDevice(
        id: *mut Uuid,
        backing_hdl: HandlePtr<IOHandle>,
        cfg: *const BlockDeviceConfiguration,
        ns: HandlePtr<NamespaceHandle>,
    ) -> SysResult;
    /// Removes the block device backed by `backing_hdl`, with the specific configuration
    ///
    ///
    ///
    /// ## Errors
    ///
    /// If `backing_hdl` is not a valid `IOHandle`, returns `INVALID_HANDLE`.
    ///
    /// If `backing_hdl` was not previously used in a call to [`CreateBlockDevice`] or the device was subsequently removed by another call to this funciton,
    ///  returns `INVALID_STATE`.
    ///
    ///
    pub fn RemoveBlockDevice(backing_hdl: HandlePtr<IOHandle>) -> SysResult;
    /// Creates a new character device, backed by a given `IOHandle`.
    ///
    /// Character devices are not seekable or random access - `backing_hdl` may be non-seekable (Does not have `CHAR_SEEKABLE`), and handles referring to it will not have the characteristics `CHAR_SEEKABLE` or `CHAR_RANDOM_ACCESS`, regardless of the underlying handle
    ///
    ///
    /// The readability and writability of the device are those of the `IOHandle`,
    ///  but are limited by the permissions of the process that opens the device and the `acl` specified in `cfg`.
    ///
    /// `id` can be assigned by either the process or the kernel. If `id` is set to the nil UUID (all zeroes), the kernel will generate a device id and store it in `id`.
    ///  Otherwise, the device is assigned the `id` specified in `id` if it is unused.
    ///
    /// If `ns` is specified, then the device is created inside that namespace only. Otherwise, the device is created in the device scope of the current thread.
    /// ## Errors
    ///
    /// If `backing_hdl` is not a valid `IOHandle`, returns `INVALID_HANDLE`.
    ///
    /// If `backing_hdl` was previouslly passed to either this function or [`CreateBlockDevice`] and the created device has not been remoed, returns `DEVICE_UNAVAILABLE`.
    ///
    /// If `backing_hdl` does not have [`CHAR_SEEKABLE`], returns `INVALID_OPERATION`. If `backing_hdl` does not have [`CHAR_RANDOMACCESS`] and `cfg.base` is not set to `0`, returns `INVALID_OPERATION`.
    ///
    /// If the current thread does not have the kernel permission `CREATE_BLOCK_DEVICE`, returns `PERMISSION`.
    ///
    /// If `id` is set to an explicit ID, `ns` is not an explicit namespace handle that has devices isolated, and the current thread does not have `ASSIGN_DEVICE_ID` permission,
    ///  returns `PERMISSION`.
    ///
    /// If `id` is set to an explicit ID, and that id is already in use by a device within the device scope of the specified namespace, returns `ALREADY_EXISTS`.
    ///
    pub fn CreateCharDevice(
        id: *mut Uuid,
        backing_hdl: HandlePtr<IOHandle>,
        cfg: *const CharDeviceConfiguration,
    ) -> SysResult;
    /// Removes the character device backed by `backing_hdl`.
    /// ## Errors
    ///
    /// If `backing_hdl` is not a valid `IOHandle`, returns `INVALID_HANDLE`.
    ///
    /// If `backing_hdl` was not previously used in a call to [`CreateBlockDevice`] or the device was subsequently removed by another call to this funciton,
    ///  returns `INVALID_STATE`.
    ///
    pub fn RemoveCharDevice(backing_hdl: HandlePtr<IOHandle>) -> SysResult;

    /// Opens a device by it's id, if the given device exists.
    ///
    /// ## Errors
    ///
    /// If `id` does not identify a valid device, returns `UNKNOWN_DEVICE`
    ///
    ///
    pub fn OpenDevice(hdl: *mut HandlePtr<DeviceHandle>, id: Uuid) -> SysResult;
    pub fn CloseDevice(hdl: HandlePtr<DeviceHandle>) -> SysResult;

    pub fn GetDeviceLabel(hdl: HandlePtr<DeviceHandle>, label: *mut KStrPtr) -> SysResult;
    pub fn GetOptimisticIOSize(hdl: HandlePtr<DeviceHandle>, io_size: *mut u64) -> SysResult;
    pub fn GetDeviceId(hdl: HandlePtr<DeviceHandle>, id: *mut Uuid) -> SysResult;

    pub fn GetFileDeviceLabel(hdl: HandlePtr<FileHandle>, label: *mut KStrPtr) -> SysResult;
    pub fn GetFileOptimisticIOSize(hdl: HandlePtr<FileHandle>, io_size: *mut u64) -> SysResult;
    pub fn GetFileDeviceId(hdl: HandlePtr<FileHandle>, id: *mut Uuid) -> SysResult;
    pub fn OpenDeviceFromFile(
        devhdl: *mut HandlePtr<DeviceHandle>,
        file: HandlePtr<FileHandle>,
    ) -> SysResult;

    /// Issues a Command to a device. The supported commands are device specific, and the parameters for each command is command specific
    pub fn IssueDeviceCommand(hdl: HandlePtr<DeviceHandle>, cmd: *const Uuid, ...) -> SysResult;

    pub fn MountFilesystem(
        resolution_base: HandlePtr<FileHandle>,
        path: KStrCPtr,
        devid: Uuid,
        opts: *const MountOptions,
    ) -> SysResult;

    pub fn RegisterDeviceCommand(
        devid: *const Uuid,
        cmdid: *mut Uuid,
        callback: unsafe extern "C" fn(
            cmdid: *const Uuid,
            callctx: HandlePtr<SecurityContext>,
            ...
        ) -> SysResult,
        callback_stack: *mut c_void,
        sigtys: *const DeviceCommandParameter,
        param_count: c_ulong,
    ) -> SysResult;
}
