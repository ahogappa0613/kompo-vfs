use std::{
    cell::OnceCell,
    collections::HashMap,
    ffi::{c_char, c_int, c_long, CStr, CString, OsStr},
    path::{Path, PathBuf},
};

static mut FS_DATA: OnceCell<Fs> = OnceCell::new();
type VALUE = u64;
extern "C" {
    static PATH_ARRAY: u8;
    static PATH_ARRAY_SIZE: u64;
    static START_AND_END: u64;
    static START_AND_END_SIZE: u64;
    static FILES: u8;
    static FILES_SIZE: u64;
    static LOAD_PATHS: u8;
    static LOAD_PATHS_SIZE: u64;

    static rb_cObject: VALUE;
    fn rb_define_class(name: *const c_char, rb_super: VALUE) -> VALUE;
    fn rb_string_value_ptr(v: *const VALUE) -> *const c_char;
    fn rb_define_singleton_method(
        object: VALUE,
        name: *const c_char,
        func: unsafe extern "C" fn(v: VALUE, v2: VALUE) -> VALUE,
        argc: c_int,
    );
    fn rb_str_new(ptr: *const c_char, len: c_long) -> VALUE;
    fn rb_str_new_cstr(ptr: *const c_char) -> VALUE;
    fn rb_ary_new_from_values(n: c_long, elts: *const VALUE) -> VALUE;
}

#[derive(Debug)]
struct Fs<'a> {
    path_map: Box<HashMap<&'a Path, usize>>,
    start_and_end: &'a [u64],
    files: &'a [u8],
}

impl<'a> Fs<'a> {
    pub fn from(
        path_map: Box<HashMap<&'a Path, usize>>,
        start_and_end: &'a [u64],
        files: &'a [u8],
    ) -> Self {
        Self {
            path_map,
            start_and_end,
            files,
        }
    }

    pub fn get_file(&self, path: &Path) -> Option<&'a CStr> {
        if let Some(index) = self.path_map.get(path) {
            self.get_file_with_index(*index)
        } else {
            None
        }
    }

    pub fn get_file_with_index(&self, index: usize) -> Option<&'a CStr> {
        let mut start_and_end = self.start_and_end.iter().skip(index).take(2);
        if let (Some(start), Some(end)) = (start_and_end.next(), start_and_end.next()) {
            let (start, end) = (*start as usize, *end as usize);
            Some(CStr::from_bytes_with_nul(&self.files[start..end]).unwrap())
        } else {
            None
        }
    }

    pub fn get_file_name_with_index(&self, index: usize) -> &'a [u8] {
        let path_array = unsafe { std::slice::from_raw_parts(&PATH_ARRAY, PATH_ARRAY_SIZE as _) };
        let splited_array = path_array.split(|char| *char == b',').collect::<Vec<_>>();

        splited_array[index]
    }
}

enum Ruby {
    FALSE = 0x00,
    NIL = 0x04,
    TRUE = 0x14,
}

#[no_mangle]
pub unsafe extern "C" fn get_patch_require() -> *const c_char {
    let data = FS_DATA.get_or_init(set_fs);
    let binding = Path::new("/root/patch_require.rb");
    let script = data.get_file(binding).expect("Not found pacth_require.rb");

    script.as_ptr() as *const _
}

#[no_mangle]
pub unsafe extern "C" fn get_start_file_name() -> *const u8 {
    let data = FS_DATA.get().unwrap();

    data.get_file_name_with_index(0).as_ptr()
}

unsafe extern "C" fn get_start_file_name_func(_: VALUE, _: VALUE) -> VALUE {
    let data = FS_DATA.get().unwrap();

    rb_str_new_cstr(data.get_file_name_with_index(0).as_ptr() as *const _)
}

unsafe extern "C" fn get_start_file_script_func(_: VALUE, _: VALUE) -> VALUE {
    let data = FS_DATA.get().unwrap();

    if let Some(name) = data.get_file_with_index(0) {
        rb_str_new_cstr(name.as_ptr())
    } else {
        Ruby::NIL as VALUE
    }
}

unsafe extern "C" fn get_load_paths_func(_: VALUE, _: VALUE) -> VALUE {
    let data = unsafe {
        String::from_utf8_lossy(std::slice::from_raw_parts(
            &LOAD_PATHS,
            LOAD_PATHS_SIZE as usize,
        ))
        .to_string()
    };

    let paths: Vec<VALUE> = data
        .split(|str| str == ',')
        .map(|path| unsafe { rb_str_new(path.as_ptr() as *const c_char, path.len() as i64) })
        .collect();

    unsafe { rb_ary_new_from_values(paths.len() as c_long, paths.as_ptr()) }
}

unsafe extern "C" fn get_file_from_fs_func(_: VALUE, rb_path: VALUE) -> VALUE {
    let rb_path = rb_string_value_ptr(&rb_path);
    let rb_path = CStr::from_ptr(rb_path);

    let path = std::path::Path::new(rb_path.to_str().unwrap());

    let mut norm_path = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::Prefix(_) => todo!(), // for windows
            std::path::Component::RootDir => norm_path.push("/"),
            std::path::Component::CurDir => {
                // nothing to do
            }
            std::path::Component::ParentDir => {
                norm_path.pop();
            }
            std::path::Component::Normal(name) => {
                norm_path.push(name);
            }
        }
    }

    let data = unsafe { FS_DATA.get().unwrap() };

    if let Some(script) = data.get_file(&norm_path) {
        rb_str_new_cstr(script.as_ptr() as *const c_char)
    } else {
        Ruby::NIL as VALUE
    }
}

fn set_fs() -> Fs<'static> {
    // initialize fs
    let mut path_map = HashMap::new();
    let path_array = unsafe { std::slice::from_raw_parts(&PATH_ARRAY, PATH_ARRAY_SIZE as _) };
    let splited_array = path_array.split(|char| *char == b',');

    for (i, bytes) in splited_array.enumerate() {
        let string =
            std::ffi::CStr::from_bytes_with_nul(bytes).expect("Not null terminated string");
        let string = unsafe { OsStr::from_encoded_bytes_unchecked(string.to_bytes()) };
        let path = std::path::Path::new(string);
        path_map.insert(path, i);
    }

    let start_and_end =
        unsafe { std::slice::from_raw_parts(&START_AND_END, START_AND_END_SIZE as _) };
    let files = unsafe { std::slice::from_raw_parts(&FILES, FILES_SIZE as _) };

    Fs::from(Box::new(path_map), start_and_end, files)
}

#[no_mangle]
pub unsafe extern "C" fn Init_fs() {
    // define ruby class
    let c_name = CString::new("Fs").unwrap();
    let get_start_file_script = CString::new("get_start_file_script").unwrap();
    let get_start_file_name = CString::new("get_start_file_name").unwrap();
    let get_load_paths = CString::new("get_load_paths").unwrap();
    let get_file_from_fs = CString::new("get_file_from_fs").unwrap();

    let class = rb_define_class(c_name.as_ptr(), rb_cObject);
    rb_define_singleton_method(
        class,
        get_start_file_name.as_ptr(),
        get_start_file_name_func,
        0,
    );

    rb_define_singleton_method(
        class,
        get_start_file_script.as_ptr(),
        get_start_file_script_func,
        0,
    );

    rb_define_singleton_method(class, get_load_paths.as_ptr(), get_load_paths_func, 0);

    rb_define_singleton_method(class, get_file_from_fs.as_ptr(), get_file_from_fs_func, 1);
}
