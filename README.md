# AeroCheetahDBM 🐆

A high-performance, lightweight desktop database management client built with **Tauri**, **Vue 3**, and **Rust**. Designed for speed, security, and a seamless developer experience across multiple database systems.

<div align="center">
  <img src="https://via.placeholder.com/200x200.png?text=AeroCheetahDBM+Logo" alt="AeroCheetahDBM Logo" width="160" />
  <p><i>The fastest database explorer in the ecosystem.</i></p>
</div>

## 🚀 Key Features

### 🔌 Multi-Database Support
Connect to various database systems using a single unified interface:
- **PostgreSQL:** Full support via `sqlx`.
- **MySQL:** High-performance connection via `sqlx`.
- **SQL Server:** Native support for TDS protocol via `tiberius`.
- **MongoDB:** NoSQL exploration and command execution via the official Rust driver.
- **Mock DB:** Built-in provider for testing and demos without external dependencies.

### 📝 Advanced SQL Editor
A professional-grade editing experience powered by **Monaco Editor** (the engine behind VS Code):
- **Multi-Tab Support:** Manage multiple queries simultaneously.
- **Intelligent Autocomplete:** Table and column name suggestions based on your active schema.
- **SQL Formatting:** Keep your queries clean with built-in formatting (`Shift+Alt+F`).
- **Partial Execution:** Select a block of text to execute only that specific part.
- **File System Integration:** Open and save SQL files directly to your local disk.

### 🛠️ Connection Management
- **Persistence:** Save and organize your most-used connections securely.
- **Quick Edit:** Easily update connection details (Host, Port, User, etc.) or change aliases.
- **Flexible Fields:** Support for both structured forms and raw connection strings.
- **Default Ports:** Automatically detects and sets standard ports for different DB types.

### ⚙️ User Settings & Performance
- **Idle Timeout:** Automatically disconnect from databases after a period of inactivity (configurable from 5 min to 48 hours).
- **Workspace Organization:** Set a default workspace folder to automatically organize saved queries by database alias.
- **Dark Mode:** A sleek, VS Code-inspired interface designed for long coding sessions.

## 🛠️ Tech Stack

- **Backend:** Rust, Tauri v2, Tokio, SQLx, Tiberius, MongoDB Rust Driver.
- **Frontend:** Vue 3 (Composition API), TypeScript, Pinia (State Management), Vite.
- **Components:** Monaco Editor, Lucide Vue Next Icons, SQL Formatter.

## 🚦 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

### Installation
1. Clone the repository.
2. Install dependencies:
   ```bash
   npm install
   ```

### Development
Run the app in development mode:
```bash
npm run tauri dev
```

### Build (Portable .exe)
To create a single-file portable executable for Windows:
1. Ensure `tauri.conf.json` has `installerMode: "portable"` in the NSIS settings.
2. Run:
   ```bash
   npm run tauri build
   ```

## 📂 Project Structure

- `src/`: Frontend Vue application.
- `src-tauri/`: Backend Rust application and Tauri configuration.
- `public/`: Static assets.

## 📄 License
This project is licensed under the MIT License - see the LICENSE file for details.
