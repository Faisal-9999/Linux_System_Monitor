# Linux System Monitor - Complete Guide

## 📋 Table of Contents
1. [Project Overview](#project-overview)
2. [Architecture & Data Flow](#architecture--data-flow)
3. [Technology Stack](#technology-stack)
4. [Component Breakdown](#component-breakdown)
5. [How It Works (Bottom-Up)](#how-it-works-bottom-up)
6. [Setup & Running](#setup--running)
7. [Database Schema](#database-schema)
8. [GUI Features](#gui-features)
9. [Key Design Decisions](#key-design-decisions)

---

## Project Overview

**Linux System Monitor** is a real-time desktop application that captures system performance metrics (CPU, RAM, disk, network) and per-process resource usage, visualizes them in a live GUI, and persists historical data to PostgreSQL.

### Core Features
- **Per-Core CPU Visualization**: Real-time multi-line graph showing each CPU core's utilization (0-100%)
- **System Statistics**: Overall CPU, RAM usage, disk I/O speeds (MB/s), and network speeds (KB/s)
- **Process Monitor**: Scrollable table showing all running processes with PID, CPU%, RAM, state, and thread count
- **Database Persistence**: Process data saved every ~3 seconds to PostgreSQL for historical tracking
- **Non-Blocking GUI**: Background thread produces system snapshots asynchronously while UI remains responsive

---

## Architecture & Data Flow

### System Diagram
```
┌─────────────────────────────────────────────────────────────┐
│                         GUI (eframe/egui)                     │
│  Per-Core CPU Graph | System Stats | Process Table           │
└──────────────────────────────┬──────────────────────────────┘
                               │
                ┌──────────────┼──────────────┐
                │              │              │
    ┌───────────▼──────┐  ┌───▼────────┐  ┌──▼────────────┐
    │  CPU/RAM/NET     │  │ /proc FS   │  │ Disk I/O      │
    │  (sysinfo lib)   │  │ (Process   │  │ (sysinfo)     │
    │                  │  │  parsing)  │  │               │
    └───────────┬──────┘  └───┬────────┘  └──┬────────────┘
                │              │              │
                └──────────────┼──────────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Background Thread  │
                    │  (SystemData::      │
                    │   start_snapshot_   │
                    │   thread)           │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  PostgreSQL         │
                    │  Process_Table      │
                    └─────────────────────┘
```

### Data Flow (Bottom-Up)

1. **OS Layer** → `/proc` filesystem (Linux kernel interface)
   - Exposes process info in `/proc/<pid>/status`
   - Exposes disk/network counters in `/proc/diskstats`, `/proc/net/dev`

2. **Parsing Layer** → `log_parser.rs`
   - `read_folder_names()`: Scans `/proc` for numeric directories (PIDs)
   - `process_data_definer()`: Parses `/proc/<pid>/status` into `ProcessData` structs
   - `cpu_usage_calculator()`: Uses `sysinfo` to compute per-process CPU % once per refresh

3. **System Metrics Layer** → `system_data.rs`
   - `SystemData` struct: Wraps `sysinfo::System`, `Networks`, `Disks`
   - `system_usage_stream()`: Iterator that sleeps 1 second, refreshes counters, yields `Snapshot`
   - `start_snapshot_thread()`: Spawns background thread running the stream, sends `Snapshot`s via MPSC channel

4. **Database Layer** → `database_connect.rs`
   - `PostgresConnector`: Manages connection to PostgreSQL
   - `init_table()`: Creates `Process_Table` if missing (schema defined in code)
   - `save_data(Vec<ProcessData>)`: Upserts process rows inside a transaction (every ~3s)
   - `get_table_data()`: Reads latest rows from DB, maps back to `ProcessData`

5. **GUI & Orchestration Layer** → `app.rs`
   - `LinuxApp`: Main app state
   - Stores: histories (CPU, RAM, disk, network), current process list, snapshot receiver
   - Every frame (~200ms): Calls `update_system_metrics()`
     - Refreshes per-core CPU (fast)
     - Computes per-process CPU usage
     - **Drains snapshot receiver** for RAM/disk/network (background thread provides these)
     - Stores values in histories (keep last 60 samples)
   - Every 3 frames (~600ms-900ms): DB operations
     - Parses `/proc`, merges CPU usage, calls `save_data(...)`
     - Calls `get_table_data()`, updates `process_data` vec for UI display
   - Renders per-core graph, stats grid, process table using egui

---

## Technology Stack

### Language & GUI
- **Rust** (stable toolchain, 2024 edition)
- **eframe**: Window/event loop library
- **egui**: Immediate-mode GUI framework
- **egui_plot**: 2D line graph plotting

### System Interaction
- **sysinfo**: Cross-platform system metrics (CPU, RAM, disk, network)
- **/proc filesystem**: Direct Linux kernel interface (processes, disk stats)

### Database
- **PostgreSQL**: Persistent storage
- **postgres crate**: Rust PostgreSQL client
- Connection string: `postgres://faisal:Fajita2231@localhost:5432/db_project`

### Concurrency
- **std::sync::mpsc**: Multi-producer, single-consumer channel (background thread ↔ GUI)
- **std::thread**: Spawns background snapshot producer

---

## Component Breakdown

### 1. `main.rs`
**Purpose**: Entry point  
**Responsibility**: Calls `LinuxApp::start_app()` to initialize and run the GUI  
**Key Function**:
```rust
fn main() -> eframe::Result<()> {
    let app = LinuxApp::default();
    app.start_app()
}
```

### 2. `system_data.rs`
**Purpose**: Background snapshot producer and system metric aggregation  
**Structs**:
- `SystemData`: Holds `sysinfo::System`, `Networks`, `Disks`
- `Snapshot`: Contains aggregated metrics (CPU, RAM, network speeds, disk I/O speeds)

**Key Functions**:
- `initialize()`: Creates fresh `SystemData`
- `system_usage_stream()`: Iterator that:
  - Sleeps 1 second between samples
  - Refreshes all metrics
  - Calculates **disk speeds** as (current_bytes - previous_bytes) / 1 sec / 1024² → MB/s
  - Clamps negative speeds to 0 (handle kernel counter resets)
  - Yields `Snapshot` with: overall CPU%, RAM (MiB), net down/up (KB/s), disk read/write speeds (MB/s)
- `start_snapshot_thread()`: Spawns thread, runs iterator, sends snapshots over MPSC channel

**Design Note**: The background thread produces snapshots every 1 second without blocking the GUI. UI drains the receiver asynchronously.

### 3. `log_parser.rs`
**Purpose**: Parse /proc filesystem and compute per-process CPU  
**Structs**:
- `ProcessData`: Holds PID, PPID, name, CPU%, RAM, state, threads

**Key Functions**:
- `read_folder_names()`: Lists `/proc` directories, returns Vec of numeric strings (PIDs)
- `process_data_definer(file)`: Parses `/proc/<pid>/status` file into `ProcessData`
- `cpu_usage_calculator(system)`: Uses `sysinfo::System` to sample CPU info, returns HashMap<PID, CPU%>
  - **Optimization**: Called once per refresh instead of per-process to avoid repeated expensive calls

### 4. `process_data.rs`
**Purpose**: Data type for process information  
**Struct**:
```rust
pub struct ProcessData {
    pub pid: u32,
    pub ppid: u32,
    pub process_name: String,
    pub cpu_usage: f64,        // % (0-100+)
    pub ram_usage: f64,        // KB
    pub state: String,         // e.g., "S(sleeping)", "R(running)"
    pub threads_used: u32,
}
```

### 5. `database_connect.rs`
**Purpose**: PostgreSQL connection and persistence  
**Struct**:
- `PostgresConnector`: Manages DB client

**Key Functions**:
- `default()`: Opens connection using hardcoded credentials
- `init_table()`: Creates `Process_Table(pid PRIMARY KEY, ppid, name, cpu_usage DOUBLE PRECISION, ram_usage DOUBLE PRECISION, state TEXT, threads INT)`
- `save_data(processes)`: Upserts rows into table (all in single transaction) every ~3 seconds
- `get_table_data()`: Selects all rows, maps to `ProcessData` vec

**Important**: Upsert uses `ON CONFLICT(pid) DO UPDATE` so same PIDs are updated, not duplicated.

### 6. `app.rs`
**Purpose**: Main application state and GUI rendering  
**Struct**:
```rust
pub struct LinuxApp {
    system_data: SystemData,          // sysinfo wrappers
    db_connector: Option<PostgresConnector>,
    process_data: Vec<ProcessData>,   // latest from DB
    
    // Histories (keep last 60 samples)
    cpu_per_core_history: Vec<Vec<f64>>,
    ram_history: Vec<f64>,
    net_down_history: Vec<f64>,
    net_up_history: Vec<f64>,
    disk_read_history: Vec<f64>,
    disk_write_history: Vec<f64>,
    
    last_disk_read: f64,
    last_disk_write: f64,
    last_net_down: f64,
    last_net_up: f64,
    
    cpu_process_usage: HashMap<u32, f64>,  // from cpu_usage_calculator
    snapshot_rx: Receiver<Snapshot>,       // channel from background thread
    
    prev_disk_read_bytes: u64,
    prev_disk_write_bytes: u64,            // for fallback speed calculation
    frame_counter: u32,                    // throttle DB ops every 3 frames
}
```

**Key Methods**:
- `update_system_metrics()`:
  1. Increment frame counter
  2. Refresh CPU per-core
  3. Compute per-process CPU usage
  4. **Drain snapshot receiver** → use latest snapshot for RAM/network/disk (prevents UI blocking)
  5. Fallback to inline refresh if no snapshot available
  6. Every 3 frames: parse /proc, save to DB, fetch latest rows
- `draw_per_core_graph()`: Line plot using egui_plot (8 colors for cores)
- `draw_system_stats()`: 3-column grid showing CPU, RAM, disk, network
- `draw_process_table()`: Scrollable table with columns: PID, PPID, name, CPU%, RAM, state, threads

---

## How It Works (Bottom-Up)

### Initialization (App Start)
1. `LinuxApp::default()` creates:
   - `SystemData` instance (fresh sysinfo)
   - `PostgresConnector` (connects to DB, creates table)
   - **Starts background snapshot thread** via `SystemData::start_snapshot_thread()`
   - Initializes empty histories and process data

### Every Frame (~200ms)
1. GUI calls `app.update()` (eframe callback)
2. `update_system_metrics()` runs:
   - Refresh CPU cores
   - Compute per-process CPU
   - **Try to drain snapshot receiver**:
     - If new snapshot arrived: use it for RAM/disk/network (no I/O blocking)
     - Else: fallback to inline refresh (slower but guaranteed data)
   - Append to histories, keep last 60 samples
3. Render:
   - Per-core CPU lines
   - System stats grid
   - Process table (from last DB fetch)
4. Request repaint after 200ms

### Every 3 Frames (~600-900ms)
1. Parse `/proc` for all PIDs
2. For each PID: open `/proc/<pid>/status`, parse into `ProcessData`
3. Merge CPU usage from `cpu_process_usage` map
4. Call `save_data(collected_data)` → inserts/upserts all rows in 1 transaction
5. Call `get_table_data()` → reads latest from DB
6. Update `self.process_data` for UI display

### Background Thread (Every 1 second)
1. Spawned once at app init
2. Creates own `SystemData` instance
3. Runs `system_usage_stream()`:
   - Sleep 1 sec
   - Refresh all metrics
   - Calculate disk speeds as deltas: (current_bytes - previous_bytes) / 1 sec / 1024² → MB/s
   - Clamp negative speeds to 0 (handle kernel counter resets)
   - Send `Snapshot` over channel
4. Loop repeats until channel receiver drops (app exits)

---

## Setup & Running

### Prerequisites
1. **Rust toolchain** (stable):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **PostgreSQL** (local server):
   ```bash
   # Debian/Ubuntu
   sudo apt install postgresql postgresql-contrib
   sudo systemctl start postgresql
   ```

3. **System libraries** (for eframe/wgpu):
   ```bash
   # Debian/Ubuntu
   sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
   ```

### Database Setup
1. Create user and database:
   ```bash
   sudo -u postgres psql
   # In psql:
   CREATE USER faisal WITH PASSWORD 'Fajita2231';
   CREATE DATABASE db_project OWNER faisal;
   \q
   ```

2. Or change credentials in `src/database_connect.rs`:
   ```rust
   let conn_string = "postgres://YOUR_USER:YOUR_PASS@localhost:5432/YOUR_DB";
   ```

### Build & Run
```bash
cd /home/faisal/GIGA_DRIVE/Linux_Task_Manager

# Debug build (faster compile, slower runtime)
cargo run

# Release build (slower compile, ~10x faster runtime)
cargo run --release
```

**Expected Output**:
- GUI window opens (1400x1000)
- Per-core CPU graph updates in real-time
- System stats refresh every frame
- Process table updates every ~3 seconds
- No console errors (only ignored DB warnings for non-existent processes)

---

## Database Schema

### Process_Table
```sql
CREATE TABLE IF NOT EXISTS Process_Table (
    pid INTEGER PRIMARY KEY,
    ppid INTEGER,
    name TEXT,
    cpu_usage DOUBLE PRECISION,
    ram_usage DOUBLE PRECISION,
    state TEXT,
    threads INTEGER
)
```

### Key Design Notes
- **Primary Key**: `pid` (uniquely identifies a process)
- **Upsert Strategy**: `ON CONFLICT(pid) DO UPDATE SET ...` ensures:
  - New processes are inserted
  - Existing processes are updated (not duplicated)
  - Database grows only with number of unique processes ever seen
- **Data Lifetime**: Rows persist until manually deleted; no automatic cleanup

### Query Latest Process Data
```sql
SELECT * FROM Process_Table ORDER BY cpu_usage DESC LIMIT 10;
```

---

## GUI Features

### Per-Core CPU Performance
- **Graph**: Line plot with 60 time samples, one line per CPU core
- **Colors**: Red, Blue, Green, Yellow, Light Blue, Light Green, Orange, Pink (cycling)
- **Y-axis**: 0-100% CPU utilization
- **X-axis**: Time samples (0-60)
- **Update**: Every frame

### System Statistics
- **CPU Usage**: Global CPU% (red text)
- **RAM Usage**: Used / Total in GB (blue text)
- **Disk Read**: Current read speed in MB/s (green text)
- **Disk Write**: Current write speed in MB/s (orange text)
- **Network Down**: Download speed in KB/s (light blue text)
- **Network Up**: Upload speed in KB/s (light green text)

### Per-Process Table
- **Columns**: PID, PPID, Process Name, CPU %, RAM (KB), State, Threads
- **Scrollable**: Max height 400px
- **Colors**:
  - CPU > 50%: Red
  - CPU 25-50%: Yellow
  - CPU < 25%: Green
- **Data Source**: PostgreSQL (fetched every ~3 seconds)
- **Empty State**: Shows warning if no data available

---

## Key Design Decisions

### 1. Background Thread for System Snapshots
**Problem**: Calling `sysinfo` refresh + parsing on every GUI frame blocks the UI.  
**Solution**: Spawn background thread running `system_usage_stream()` that sends snapshots every 1s. GUI drains receiver asynchronously.  
**Result**: UI always responsive, system metrics update at ~1s cadence.

### 2. Throttled Database Operations (Every 3 Frames)
**Problem**: Parsing /proc and writing to DB every frame is expensive.  
**Solution**: Only parse/save every 3 frames (~600-900ms).  
**Result**: Process list shown with slight lag but DB I/O doesn't stall GUI.

### 3. Disk Speed Calculation (Deltas, Not Totals)
**Problem**: sysinfo returns cumulative bytes since boot (10GB+). Not useful for "current speed".  
**Solution**: Track previous byte counters, calculate delta per second: `(current - prev) / 1024² = MB/s`.  
**Result**: Display actual disk I/O speed. Clamp to 0 if negative (kernel counter resets).

### 4. Per-Process CPU Once Per Refresh
**Problem**: Calling `sysinfo` CPU calculation per process is expensive.  
**Solution**: Call `cpu_usage_calculator()` once, get HashMap<PID, CPU%>, reuse for all processes.  
**Result**: ~8x faster process enumeration.

### 5. Upsert Instead of Insert
**Problem**: Process PIDs reused after exit; inserting new rows creates duplicates.  
**Solution**: Use `ON CONFLICT(pid) DO UPDATE` in SQL.  
**Result**: One row per PID, always latest data.

---

## Presentation Summary for Professor

**"This Linux System Monitor is a real-time desktop application built in Rust that visualizes system performance and process metrics. Here's how it works from the bottom up:**

1. **OS Level**: The Linux kernel exposes process and I/O data in `/proc` filesystem.

2. **Parsing Layer**: We read `/proc/<pid>/status` files and parse them into structured data. We also sample CPU and disk/network metrics via the `sysinfo` library.

3. **Background Metrics Thread**: To keep the GUI responsive, we spawn a background thread that runs a 1-second loop: it refreshes system metrics, calculates disk I/O *speeds* (not totals) by computing deltas, and sends snapshots over a channel.

4. **Database Layer**: Every ~3 seconds, we parse all processes, merge their CPU usage, and upsert the batch into PostgreSQL. This creates a persistent record without duplicates (using `ON CONFLICT`).

5. **GUI Layer**: The app uses egui for immediate-mode rendering. Each frame, it drains the snapshot channel for latest system stats (non-blocking), and periodically fetches process data from the database. We display:
   - Per-core CPU graph (8 lines, real-time)
   - System stats grid (CPU%, RAM, disk speeds, network speeds)
   - Process table from DB (scrollable, color-coded by CPU usage)

**Key Design**: Background thread keeps system metrics flowing asynchronously. Throttled DB operations (every ~600ms) keep I/O off the main thread. We optimize per-process CPU sampling by doing it once per refresh instead of per-process."**

---

## Troubleshooting

### "could not connect to server" error
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql
sudo systemctl start postgresql

# Test connection
psql -U faisal -d db_project -h localhost
```

### Permission errors reading /proc
```bash
# Run with sudo
sudo cargo run --release
```

### Negative disk speeds (-0.05 MB)
This is handled by clamping to 0 in code. Happens when kernel I/O counters reset or fluctuate.

### Process table shows "No process data available"
- Wait 3 seconds for first DB fetch
- Verify PostgreSQL is running and credentials are correct
- Check that `Process_Table` was created: `psql -U faisal -d db_project -c "\dt"`

---

**Version**: 0.1.0  
**Last Updated**: January 6, 2026
