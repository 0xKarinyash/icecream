# IceCream - A CreamAPI-Like DLC Unlocker for macOS (EXPERIMENTAL)

**⚠️ ATTENTION: THIS IS AN EXPERIMENTAL VERSION ⚠️**

> This current version is confirmed to work **only** with Hearts Of Iron IV. It should work with other games. But i'll test that when finish "stable" version also its is confirmed to work **ONLY** when launching the game executable directly from the terminal. Launching the game via the official Paradox Launcher may not work correctly or could lead to unexpected issues. Use at your own risk.

---

### What is this?
**IceCream** is a `libsteam_api.dylib` proxy library that unlocks DLCs just like creamlinux, but for macOS and in Rust. It works by intercepting calls from the game to the Steam API and reporting that all DLCs listed in the `icecream.ini` file are owned by the user.

### Features

*   **Proxy-based:** Acts as a wrapper around the original Steam API library, ensuring compatibility.
*   **Configurable:** A simple `icecream.ini` file lets you control exactly which DLCs are activated.
*   **Dynamic Hooking:** Patches game functions in memory at runtime to avoid modifying game files directly.


### Installation (HOI4)

1. Clone this repo 

2. build with `cargo build --release`

3. Create `icecream.ini` 
> You can take cream_api.ini and just rename it. As long as DLC placed in [dlc] section. See example below.

3.  **Back up your original file!** Navigate to the game's installation directory and make a copy of the original `libsteam_api.dylib` file.
    *   The typical path for the Steam version is: `~/Library/Application Support/Steam/steamapps/common/Hearts of Iron IV/hoi4.app/Contents/MacOS/`

4.  Rename the original `libsteam_api.dylib` to `libsteam_api_o.dylib` (the 'o' stands for 'original').
> This is **CRUCIAL** because icecream directly talks to `libsteam_api_o.dylib`
  
5.  Place both downloaded files (`libsteam_api.dylib` and `icecream.ini`) into the same directory where `libsteam_api_o.dylib` is now located.

### How to run
1. Open the Terminal application
2. Go to game folder `cd "~/Library/Application Support/Steam/steamapps/common/Hearts of Iron IV/`
3. Run the game `./hoi4.app/Contents/MacOS/hoi4`

### Troubleshooting
The unlocker creates a log file at `~/icecream_log.txt` (in your home directory). If you encounter any issues, this file is the first place to check for errors or other diagnostic information.

### Credits

This project was heavily inspired by and based on the initial codebase of [anticitizn/creamlinux](https://github.com/anticitizn/creamlinux). A huge thank you to their foundational work.

### License

This project is licensed under the MIT License. Please see the `LICENSE` file for the full text.