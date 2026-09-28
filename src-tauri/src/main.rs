// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Evita que se abra una consola en Windows al ejecutar en modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    datasearch_lib::run();
}
