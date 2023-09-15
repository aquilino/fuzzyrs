use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;
use crossbeam::scope;
use isahc::prelude::*;

fn main() {
    // Obtener los argumentos de línea de comandos
    let args: Vec<String> = env::args().collect();

    // Analizar los argumentos
    let wordlist_file = get_argument(&args, "-w").expect("No se proporcionó el archivo de lista de palabras.");
    let target_url = get_argument(&args, "-h").expect("No se proporcionó la URL objetivo.");
    let num_threads: usize = get_argument(&args, "-t")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut extensions: Vec<String> = get_argument(&args, "-x")
        .map(|s| s.split(',').map(|ext| format!(".{}", ext)).collect())
        .unwrap_or_else(Vec::new);
    extensions.push("".to_string());  // Añadir una cadena vacía a las extensiones
    let output_file = get_argument(&args, "-o");
    let hidden_status: Vec<u16> = get_argument(&args, "-b")
        .map(|s| s.split(',').map(|code| code.parse().unwrap()).collect())
        .unwrap_or_else(Vec::new);
    let hidden_length: Option<usize> = get_argument(&args, "--hidden-length")
        .and_then(|s| s.parse().ok());

    // Leer el archivo de lista de palabras
    let wordlist = match read_wordlist(&wordlist_file) {
        Ok(wordlist) => wordlist,
        Err(e) => {
            eprintln!("Error al leer el archivo de lista de palabras: {:?}", e);
            return;
        }
    };

    // Dividir las palabras en bloques para asignar a cada hilo
    let chunk_size = (wordlist.len() as f32 / num_threads as f32).ceil() as usize;
    let chunks: Vec<_> = wordlist.chunks_exact(chunk_size).collect();

    // Imprimir encabezados de la tabla
    println!("{:<10} {:<10} {:<20} {}", "Código", "Longitud", "Archivo", "URL");

    // Crear el archivo de salida si se proporcionó
    if let Some(output_file) = &output_file {
        File::create(output_file).expect("No se pudo crear el archivo de salida");
    }

    // Iterar sobre cada bloque de palabras utilizando crossbeam para el manejo de hilos
    if let Err(e) = scope(|s| {
        for chunk in chunks {
            // Clonar los valores necesarios para cada hilo
            let target_url_clone = target_url.clone();
            let extensions_clone = extensions.clone();
            let output_file_clone = output_file.clone();
            let hidden_status_clone = hidden_status.clone();
            let hidden_length_clone = hidden_length;

            // Pasar el bloque de palabras como referencia a cada hilo
            s.spawn(move |_| {
                for word in chunk {
                    for ext in &extensions_clone {
                        // Construir la URL del objetivo
                        let url = format!("{}/{}{}", target_url_clone, word, ext);

                        // Realizar la solicitud HTTP de forma sincrónica
                        if let Ok(mut response) = isahc::get(&url) {
                            if !hidden_status_clone.contains(&response.status().as_u16()) {
                                if hidden_length_clone.map_or(true, |len| response.text().unwrap().len() != len) {
                                    println!("{:<10} {:<10} {:<20} {}", response.status(), response.text().unwrap().len(), word, url);
                                    if let Some(output_file) = &output_file_clone {
                                        write_output(output_file, &url);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }) {
        eprintln!("Error al crear hilos: {:?}", e);
    }
}

fn read_wordlist<P>(filename: P) -> io::Result<Vec<String>>
where P: AsRef<Path>,
{
let file = File::open(filename)?;
let reader = io::BufReader::new(file);
Ok(reader.lines().collect::<Result<_, _>>()?)
}

fn get_argument(args: &[String], flag: &str) -> Option<String> {
    args.iter()
    .position(|arg| arg == flag)
    .and_then(|pos| args.get(pos + 1))
    .cloned()
}

fn write_output(filename: &str, url: &str) {
    let mut file = OpenOptions::new()
    .write(true)
    .append(true)
    .open(filename)
    .expect("No se pudo abrir el archivo de salida");
    if let Err(e) = write!(file, "{}", url) {
        eprintln!("No se pudo escribir en el archivo de salida: {:?}", e);
    }
}
