use log::info;
use rbxlx_to_rojo::{filesystem::FileSystem, process_instructions};
use std::{
    borrow::Cow,
    fmt, fs,
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
enum Problem {
    BinaryDecodeError(rbx_binary::DecodeError),
    InvalidFile,
    IoError(&'static str, io::Error),
    NFDCancel,
    NFDError(String),
    XMLDecodeError(rbx_xml::DecodeError),
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Problem::BinaryDecodeError(error) => write!(
                formatter,
                "While attempting to decode the place file, at {} rbx_binary didn't know what to do",
                error,
            ),

            Problem::InvalidFile => {
                write!(formatter, "The file provided does not have a recognized file extension (expected .rbxl or .rbxm)")
            }

            Problem::IoError(doing_what, error) => {
                write!(formatter, "While attempting to {}, {}", doing_what, error)
            }

            Problem::NFDCancel => write!(formatter, "Didn't choose a file."),

            Problem::NFDError(error) => write!(
                formatter,
                "Something went wrong when choosing a file: {}",
                error,
            ),

            Problem::XMLDecodeError(error) => write!(
                formatter,
                "While attempting to decode the place file, at {} rbx_xml didn't know what to do",
                error,
            ),
        }
    }
}

struct WrappedLogger {
    log: env_logger::Logger,
    log_file: Arc<RwLock<Option<fs::File>>>,
}

impl log::Log for WrappedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.log.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.log.log(record);

            if let Some(ref mut log_file) = &mut *self.log_file.write().unwrap() {
                log_file
                    .write(format!("{}\r\n", record.args()).as_bytes())
                    .ok();
            }
        }
    }

    fn flush(&self) {}
}

fn routine() -> Result<(), Problem> {
    let env_logger = env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .build();

    let log_file = Arc::new(RwLock::new(None));
    let logger = WrappedLogger {
        log: env_logger,
        log_file: Arc::clone(&log_file),
    };

    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    info!("rbxl-to-rojo {}", env!("CARGO_PKG_VERSION"));

    info!("Select a place file.");
    let file_path = PathBuf::from(match std::env::args().nth(1) {
        Some(text) => text,
        None => match nfd::open_file_dialog(Some("rbxl,rbxm"), None)
            .map_err(|error| Problem::NFDError(error.to_string()))?
        {
            nfd::Response::Okay(path) => path,
            nfd::Response::Cancel => Err(Problem::NFDCancel)?,
            _ => unreachable!(),
        },
    });

    info!("Opening place file");
    let mut file = fs::File::open(&file_path)
        .map_err(|error| Problem::IoError("read the place file", error))?;
    
    // Check if file is actually XML format (starts with <roblox and is valid XML)
    let mut header = [0u8; 20];
    file.read_exact(&mut header)
        .map_err(|error| Problem::IoError("read file header", error))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| Problem::IoError("seek file", error))?;
    
    // Check for valid XML: should start with <?xml or <roblox (without !)
    let is_xml = header.starts_with(b"<?xml") || 
                 (header.starts_with(b"<roblox") && !header.starts_with(b"<roblox!"));
    
    let file_source = BufReader::new(file);
    info!("Decoding place file, this is the longest part...");

    let tree = if is_xml {
        // File is XML format, decode as XML
        rbx_xml::from_reader_default(file_source).map_err(Problem::XMLDecodeError)?
    } else {
        // File is binary format, decode as binary
        match file_path
            .extension()
            .map(|extension| extension.to_string_lossy())
        {
            Some(Cow::Borrowed("rbxm")) | Some(Cow::Borrowed("rbxl")) => {
                rbx_binary::from_reader(file_source).map_err(Problem::BinaryDecodeError)?
            }
            _ => return Err(Problem::InvalidFile),
        }
    };

    info!("Select the path to put your Rojo project in.");
    let root = PathBuf::from(match std::env::args().nth(2) {
        Some(text) => text,
        None => match nfd::open_pick_folder(Some(&file_path.parent().unwrap().to_string_lossy()))
            .map_err(|error| Problem::NFDError(error.to_string()))?
        {
            nfd::Response::Okay(path) => path,
            nfd::Response::Cancel => Err(Problem::NFDCancel)?,
            _ => unreachable!(),
        },
    });

    let output_dir = root.join(file_path.file_stem().unwrap());
    fs::create_dir_all(&output_dir)
        .map_err(|error| Problem::IoError("couldn't create output directory", error))?;
    
    let mut filesystem = FileSystem::from_root(output_dir.into());

    log_file.write().unwrap().replace(
        fs::File::create(root.join("rbxl-to-rojo.log"))
            .map_err(|error| Problem::IoError("couldn't create log file", error))?,
    );

    info!("Starting processing, please wait a bit...");
    process_instructions(&tree, &mut filesystem);
    info!("Done! Check rbxl-to-rojo.log for a full log.");
    Ok(())
}

fn main() {
    if let Err(error) = routine() {
        eprintln!("An error occurred while using rbxl-to-rojo.");
        eprintln!("{}", error);
    }
}
