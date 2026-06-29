pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
gnome = import('gnome')

blueprint_compiler = find_program('blueprint-compiler')

blueprints = custom_target(
  'blueprints',
  input: files(
    'window.blp',
    'shortcuts-dialog.blp',
  ),
  output: [
    'window.ui',
    'shortcuts-dialog.ui',
  ],
  command: [
    blueprint_compiler,
    'batch-compile',
    meson.current_build_dir(),
    meson.current_source_dir(),
    '@INPUT@',
  ],
)

gnome.compile_resources('drill',
  'drill.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('GETTEXT_PACKAGE', 'drill')
conf.set_quoted('LOCALEDIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('PKGDATADIR', pkgdatadir)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.project_build_root() / 'src' / 'config.rs',
  meson.project_source_root() / 'src' / 'config.rs',
  check: true
)

cargo_bin  = find_program('cargo')
cargo_home = meson.project_build_root() / 'cargo-home'
cargo_target_dir = meson.project_build_root() / 'src'
cargo_profile_arg = ''

if get_option('buildtype') == 'release'
  cargo_profile_arg = '--release'
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_build_script = 'CARGO_HOME="$1" "$2" build --manifest-path "$3" --target-dir "$4" $5 && cp "$4/$6/$7" "$8"'

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'sh', '-c', cargo_build_script, 'cargo-build',
    cargo_home,
    cargo_bin.full_path(),
    meson.project_source_root() / 'Cargo.toml',
    cargo_target_dir,
    cargo_profile_arg,
    rust_target,
    meson.project_name(),
    '@OUTPUT@',
  ]
)
