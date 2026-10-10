# fettle-spike

A small test tool for Fettle. It reads what Windows can tell about this PC (disks, battery, memory, Windows version, startup apps, graphics) and saves it to a JSON file. **It only reads. It changes nothing on the PC and sends nothing anywhere.**

Serial numbers, the PC name, your Windows username and network addresses are hidden in the file. Product keys are never read.

## How to run it

1. Copy `fettle-spike.exe` to any folder on the PC, for example the Desktop.
2. Open that folder in File Explorer, right-click an empty spot, and choose **Open in Terminal**.
3. Run it with a short name for this PC:
   ```
   .\fettle-spike.exe --label old-laptop
   ```
4. Run it a second time as administrator: open the Start menu, type **Terminal**, choose **Run as administrator**, go to the same folder (`cd` to it), and run the same command again.
5. Send back both `.json` files from the `dumps` folder that appears next to the exe.

Each run takes about 10 to 15 seconds.

## "Windows protected your PC"

The tool isn't signed, so Windows SmartScreen may warn you the first time. Click **More info**, then **Run anyway**.

**On a work laptop, check with your IT department before running it.** Company policy may not allow unsigned programs.

## The test kit (easier for other PCs)

`npm run kit` (from the repo root) builds `target/fettle-test-kit.zip`: this tool, `fettle-collect` (the app's real scan), a `run-fettle-test.bat` to double-click, and a short README. The tools are built with the C runtime linked in, so they run on a fresh Windows without installing anything.
