use std::env;
use std::io;
use std::process::Command;
use std::path::{Path, PathBuf};
use colored::*;
use dirs;
use whoami;
use std::fs::{self, DirEntry, Metadata};
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;
use chrono::DateTime;
use chrono::Local;
use std::collections::HashMap;
use std::cmp::max;
use terminal_size::{terminal_size, Width};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub struct Terminal {
    rl: DefaultEditor,
    aliases: HashMap<String, String>,
}

impl Terminal {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let rl = DefaultEditor::new()?;

        let mut terminal = Self {
            rl,
            aliases: HashMap::new(),
        };

        if let Err(e) = terminal.load_rushrc() {
            eprintln!("X error loading .rushrc {e}");
        }

        Ok(terminal)
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        'main_loop: loop {
            let prompt = self.print_prompt();
            let readline = self.rl.readline(&prompt);

            match readline {
                Ok(line) => {
                    let input = line.trim();

                    if input.is_empty() {
                        continue;
                    }

                    if !self.rl.history().iter().any(|h| *h == line) {
                        self.rl.add_history_entry(&line)?;
                    }
                    
                    let commands_chained = line.split("&&");
                    let mut last_command_success = true;

                    for command_str in commands_chained {
                        if !last_command_success {
                            break;
                        }
                    
                        let command_str = command_str.trim();
                        if command_str.is_empty() {
                            continue;
                        }
                    
                        let parts: Vec<&str> = command_str.split_whitespace().collect();

                        let expanded_parts_str: Vec<String> = self.expand_aliases(&parts);

                        let expanded_parts: Vec<&str> = expanded_parts_str.iter().map(|s| s.as_str()).collect();

                        let _command = expanded_parts[0];
                        let _args = &expanded_parts[1..];
                    
                        let parts: Vec<&str> = input.split_whitespace().collect();
                        let command = parts[0];
                        let args = &parts[1..];
                
                        match command {
                            "exit" => break 'main_loop,
                            "pwd" => {
                                match env::current_dir() {
                                    Ok(path) => println!("{}", path.display()),
                                    Err(e) => eprintln!("X {e}"),
                                }
                                last_command_success = true;
                            },
                            "cd" => {
                                let user = whoami::username().replace('"', "");

                                let target_dir = if !args.is_empty() {
                                    if args[0].starts_with("~") {
                                        args[0].replacen("~", &format!("/home/{}", user), 1).to_string()
                                    } else {
                                        args[0].to_string()
                                    }
                                } else {
                                    dirs::home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| ".".to_string())
                                };

                                if let Err(e) = env::set_current_dir(&target_dir) {
                                    eprintln!("X: {:?}: {e}", target_dir);
                                }
                                last_command_success = env::set_current_dir(&target_dir).is_ok();
                            },
                            "ls" => {
                                let path = if !args.is_empty() && !args[0].starts_with('-') {
                                    args[0]
                                } else {
                                    "."
                                };

                                let flags: Vec<&str> = args.iter()
                                    .filter(|&&arg| arg.starts_with('-')) 
                                    .copied() 
                                    .collect();
                                
                                if let Err(e) = self.colored_ls(path, &flags) {
                                    eprintln!("X: {e}");
                                }
                                last_command_success = true;
                            },
                            _ => {
                                last_command_success = self.run_command(command, args);
                            }
                        }
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    break;
                },
                Err(ReadlineError::Eof) => {
                    break;
                },
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        self.rl.save_history("history.txt")?;
        Ok(())
    }

    fn print_prompt(&self) -> String {
        let user = whoami::username().replace('"', "");
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let host = whoami::hostname();
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        
        let display_path = if let Ok(relative) = current_dir.strip_prefix(&home_dir) {
            format!("~/{}", relative.display())
        } else {
            current_dir.display().to_string()
        };
         
        format!("\n{}\n{}@{} {} ",
            display_path.bold(),
            user,
            host, 
            ">".green()
        )
    }

    fn run_command(&self, cmd: &str, args: &[&str]) -> bool {
    if cmd.is_empty() {
        return true;
    }

    match Command::new(cmd).args(args).spawn() {
        Ok(mut child) => {
            match child.wait() {
                Ok(status) => status.success(),
                Err(_) => false,
            }
        }
        Err(_) => {
            eprintln!("X: '{}'", cmd);
            false
        }
    }
}

    fn load_rushrc(&mut self) -> io::Result<()> {
        if let Some(mut path) = dirs::home_dir() {
            path.push(".rushrc");

            if path.exists() {
                let content = fs::read_to_string(path)?;

                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("#") || line.is_empty() {
                        continue;
                    }

                    if line.starts_with("alias ") {
                        let parts: Vec<&str> = line[6..].splitn(2, "=").collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().trim_matches('"').to_string();

                            self.aliases.insert(name, value);
                        }
                    } else if line.starts_with("export ") {
                        let parts: Vec<&str> = line[7..].splitn(2, '=').collect();
                        if parts.len() == 2 {
                            let key = parts[0].trim().to_string();
                            let value = parts[1].trim().trim_matches('"').to_string();

                            unsafe {
                                env::set_var(key, value);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn expand_aliases(&self, parts: &[&str]) -> Vec<String> {
        if parts.is_empty() {
            return vec![];
        }

        let command = parts[0];
        if let Some(alias_value) = self.aliases.get(command) {
            let mut expanded: Vec<String> = alias_value.split_whitespace().map(String::from).collect();
            expanded.extend(parts[1..].iter().map(|s| s.to_string()));
            expanded
        } else {
            parts.iter().map(|s| s.to_string()).collect()
        }
    }

    fn colored_ls(&self, path: &str, args: &[&str]) -> std::io::Result<()> {
        let dir = Path::new(path);
        let mut entries: Vec<_> = fs::read_dir(dir)?
            .collect::<Result<Vec<_>, _>>()?;
        
        let show_hidden = args.contains(&"-a") || args.contains(&"--all");
        let long_format = args.contains(&"-l");
        let human_readable = args.contains(&"-h");
        
        if !show_hidden {
            entries.retain(|e| !e.file_name().to_string_lossy().starts_with('.'));
        }
        
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        if long_format {
            self.ls_long_format(&entries, human_readable)?;
        } else {
            self.ls_standard_format(&entries)?;
        }

        Ok(())
    }

    fn ls_standard_format(&self, entries: &[DirEntry]) -> std::io::Result<()> {
        let term_width = if let Some((Width(w), _)) = terminal_size() {
            max(40, w as usize) 
        } else {
            80
        };

        let cols = 4;
        let col_width = term_width / cols;
        
        for (i, entry) in entries.iter().enumerate() {
            let (icon, colored_name) = self.get_file_icon_and_color(entry)?;
            let name_str = colored_name.to_string();
            let truncated_name = truncate_string(&name_str, col_width - 3); // -3 para o ícone e espaços
            
            print!("{} {:width$}", icon, truncated_name, width = col_width - 3);
            
            if (i + 1) % cols == 0 {
                println!();
            }
        }
        
        if entries.len() % cols != 0 {
            println!();
        }
        
        Ok(())
    }

    fn ls_long_format(&self, entries: &[DirEntry], human_readable: bool) -> std::io::Result<()> {
        println!("{}", "Permisions  Owner   Grup     Size    Modification       Name".bold());
        
        let term_width = if let Some((Width(w), _)) = terminal_size() {
            w as usize
        } else {
            80
        };
        let name_width = max(20, term_width - 50); // Largura mínima de 20 para o nome

        for entry in entries {
            let metadata = entry.metadata()?;
            let file_name = entry.file_name().to_string_lossy();
            
            let perms = self.format_permissions(&metadata);
            let size = if human_readable {
                self.format_size(metadata.len())
            } else {
                metadata.len().to_string()
            };
            let modified = self.format_datetime(metadata.modified()?);
            let (icon, colored_name) = self.get_file_icon_and_color(entry)?;
            
            let truncated_name = truncate_string(&colored_name.to_string(), name_width);
            
            println!("{:10} {:6} {:6} {:>8} {} {} {}",
                perms,
                metadata.uid(),
                metadata.gid(),
                size,
                modified,
                icon,
                truncated_name);
        }
        
        Ok(())
    }

    fn format_size(&self, size: u64) -> String {
        let units = ["B", "K", "M", "G", "T"];
        let mut size = size as f64;
        let mut unit_idx = 0;
        
        while size >= 1024.0 && unit_idx < units.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        
        format!("{:.1}{}", size, units[unit_idx])
    }

    fn print_entries(&self, entries: &[&DirEntry], long_format: bool) -> std::io::Result<()> {
        let mut sorted = entries.to_vec();
        sorted.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        
        let max_length = sorted.iter()
            .map(|e| e.file_name().to_string_lossy().len())
            .max()
            .unwrap_or(20);
        
        for entry in sorted {
            let (icon, colored_name) = self.get_file_icon_and_color(entry)?;
            print!("{} {:width$}  ", icon, colored_name, width = max_length);
        }
        
        Ok(())
    }

    fn get_file_icon_and_color(&self, entry: &DirEntry) -> std::io::Result<(String, String)> {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata()?;
        
        let (icon, colored_file_name) = match (metadata.file_type(), self.is_executable(&entry.path())) {
            (ft, _) if ft.is_dir() => (self.folder_icon(&file_name), file_name.blue()),
            (ft, _) if ft.is_symlink() => (" 🔗", file_name.cyan()),
            (_, true) => (" >_", file_name.green()),
            _ => {
                let color = match entry.path().extension().and_then(|s| s.to_str()) {
                    Some("jpg") | Some("png") | Some("gif") => file_name.yellow(),
                    Some("zip") | Some("gz") | Some("tar") => file_name.red(),
                    _ => file_name.white(),
                };
                (self.file_icon(&file_name), color)
            }
        };
       
        Ok((icon.to_string(), colored_file_name.to_string()))
    }

    fn format_permissions(&self, metadata: &Metadata) -> String {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            format!("{:o}", mode & 0o777)
        }
        #[cfg(not(unix))]
        {
            "----".to_string()
        }
    }

    fn format_datetime(&self, system_time: SystemTime) -> String {
        let datetime: DateTime<Local> = system_time.into();
        datetime.format("%Y-%m-%d %H:%M").to_string()
    }

    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(path) {
                return metadata.permissions().mode() & 0o111 != 0;
            }
        }
        false
    }

    fn file_icon(&self, file_name: &str) -> &str {
        if file_name.ends_with(".rs") {
            " 🦀"
        } else if file_name.ends_with(".go") {
            " 🐹"
        } else if file_name.ends_with(".c") {
            " C"
        } else if file_name.ends_with(".cpp") {
            " C++"
        } else if file_name.ends_with(".h") {
            " H"
        } else if file_name.ends_with(".py") {
            " 🐍"
        } else if file_name.ends_with(".r") {
            " 𝐑"
        } else if file_name.ends_with(".js") {
            " JS"
        } else if file_name.ends_with(".ts") {
            " TS"
        } else if file_name.ends_with(".html") {
            " 🌐"
        } else if file_name.ends_with(".css") {
            " 🎨"
        } else if file_name.ends_with(".md") {
            " "
        } else if file_name.ends_with(".json") {
            " {}"
        } else if file_name.ends_with(".toml") || file_name.ends_with(".yaml") || file_name.ends_with(".conf") || file_name.ends_with(".config") || file_name.starts_with(".") {
            " ⚙️"
        } else if file_name.ends_with(".sh") {
            " >_"
        } else if file_name.ends_with(".txt") {
            " "
        } else if file_name.ends_with(".sql") {
            " "
        } else if file_name.ends_with(".java") {
            " ☕"
        } else {
            " 📄"
        }
    }

    fn folder_icon(&self, folder_name: &str) -> &str {
        match folder_name {
            "Downloads" => " 📥",
            "Desktop" => " 🖥️",
            "Documents" | "Documentos" => " 📄",
            "Dev" | "dev" => " </>",
            "Projects" | "projects" => " 🗂️",
            "Pictures" | "Imagens" => " 🖼️",
            "Music" | "Música" => " 🎵",
            "Videos" | "Vídeos" => " 🎥",
            ".config" => " ⚙️",
            ".git" => " 🗃️",
            "node_modules" => " 📦",
            "target" => " 🛠️",
            _ => " 📁",
        }
    }

    fn relative_path(&self, base: &PathBuf, target: &str) -> String {
        let target_path = Path::new(target);

        if let Ok(relative) = target_path.strip_prefix(base) {
            relative.display().to_string()
        } else {
            target_path.display().to_string()
        }
    }
}

fn truncate_string(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else if max_width > 1 {
        let truncated: String = s.chars().take(max_width - 1).collect();
        format!("{}…", truncated)
    } else {
        "…".to_string()
    }
}
