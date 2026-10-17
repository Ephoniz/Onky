# Testing the Audio Program Cable driver on Windows

This guide walks you through installing the Audio Program virtual cable driver
on a Windows PC, checking that it works, and removing it again. No programming
is needed. Plan for about an hour the first time.

**What the driver does.** It adds virtual "cables" to Windows. Each cable is a
pair of sound devices: a playback device called **Speakers (Audio Program
Cable N)** and a recording device called **Microphone (Audio Program Cable N)**.
Whatever an app plays into the Speakers side comes straight out of the
Microphone side of the same cable, unchanged. There are 8 cables by default,
and you can choose anywhere from 1 to 16.

**Why there are extra setup steps.** This is a test build. It is signed with a
test certificate, not by Microsoft, so Windows only loads it in "test mode".
Test mode needs Secure Boot turned off. Do this on a spare PC or a virtual
machine, not on a computer you depend on.

## What you need

- Windows 10 version 2004 or newer, or Windows 11, 64-bit (x64). ARM PCs are
  not supported yet.
- An administrator account.
- A spare PC, or a virtual machine (Hyper-V, VMware or VirtualBox) with sound
  working.
- Speakers or headphones on that machine, so you can hear the result.
- The driver package: a folder containing `AudioProgramCable.sys`,
  `AudioProgramCable.inf`, `AudioProgramCable.cat`,
  `AudioProgramCableTest.cer` and a `scripts` folder. Get it from the
  `audio-program-cable-x64` artifact of the `driver` CI workflow (a zip file),
  or build it yourself with `driver\build.ps1 -RestorePackages -TestSign`.
- If BitLocker is on: your BitLocker recovery key, just in case.

## Step 1: Prepare the machine

### Option A: a virtual machine (recommended)

A virtual machine is safest, because you can undo everything with one click.

- **Hyper-V:** create a "Generation 2" VM. In the VM's **Settings > Security**,
  untick **Enable Secure Boot**. Connect with "Enhanced session" so the VM's
  sound plays on your real speakers.
- **VMware Workstation:** in **VM Settings > Options > Advanced**, untick
  **Enable secure boot** (or use BIOS firmware). Make sure a Sound Card is
  connected.
- **VirtualBox:** in **Settings > System**, untick **Enable Secure Boot** (or
  leave EFI off). In **Settings > Audio**, enable audio output and input.

Install Windows in the VM, then **take a snapshot / checkpoint** named
"before driver". If anything goes badly wrong later, restore it.

### Option B: a spare physical PC

1. Restart the PC and open its firmware setup (usually by pressing F2, Del,
   F10 or F12 while it starts; the key is often shown on screen).
2. Find **Secure Boot** (often under "Boot" or "Security") and set it to
   **Disabled**. Save and exit.
3. Start Windows. To confirm: press Win+R, type `msinfo32`, press Enter, and
   check that **Secure Boot State** says **Off**.

## Step 2: Copy the package and turn on test mode

1. Copy the driver package to the machine, for example to
   `C:\AudioProgramCable`. If it is a zip, right-click it, choose **Extract
   All**, and extract it there.
2. Click Start, type `PowerShell`, right-click **Windows PowerShell** and
   choose **Run as administrator**. Click **Yes**.
3. Type these commands, pressing Enter after each line:

   ```powershell
   cd C:\AudioProgramCable
   Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
   Get-ChildItem -Recurse | Unblock-File
   .\scripts\Enable-TestSigning.ps1
   ```

   The first two lines let this PowerShell window run the scripts. The third
   removes the "downloaded from the internet" mark from the files. The last
   one turns on test mode. If BitLocker is on, it is paused for one restart so
   you are not asked for the recovery key.

4. **Restart the computer.**
5. After the restart, the words **Test Mode** appear in the bottom-right
   corner of the desktop. That means it worked.

If the script says Secure Boot is on, go back to Step 1.

## Step 3 (optional): Smoke test with a known driver

If you want to be sure the machine itself is set up correctly, first install
the independent Sonicanvas test driver (`sonicanvas-smoke-test-x64` CI
artifact), following `smoke-test/sonicanvas/README.md`. If Sonicanvas works
and Audio Program Cable does not, the problem is in our driver. If neither
works, the problem is the machine setup. Uninstall Sonicanvas before you go on.

## Step 4: Install the driver

In the administrator PowerShell window (open a new one after the restart, and
repeat the `cd` and `Set-ExecutionPolicy` lines), run:

```powershell
.\scripts\Install-Driver.ps1
```

It trusts the test certificate, adds the driver to Windows, and creates the
device. It ends with a small table. **Status** should be **OK**, and the last
line should say "Installed".

If Windows asks "Would you like to install this device software?", click
**Install**.

Then check **Device Manager** (right-click Start > Device Manager). Under
**Sound, video and game controllers** you should see **Audio Program Virtual
Cables**, with no yellow warning triangle.

You can print a summary at any time with:

```powershell
.\scripts\Get-CableStatus.ps1
```

## Step 5: Check Sound settings

**First, make sure your real speakers are still the default.** Windows
sometimes switches the default output to a newly added device. If sound stops
coming out of your speakers, choose them again in Sound settings.

1. Open **Settings > System > Sound**.
2. Under **Output** (Windows 11), or the "Choose your output device" list
   (Windows 10), you should see 8 entries named **Speakers** with **Audio
   Program Cable 1** to **Audio Program Cable 8** underneath (some apps show
   them as "Speakers (Audio Program Cable 1)").
3. Under **Input**, you should see 8 entries named **Microphone** with **Audio
   Program Cable 1** to **Audio Program Cable 8** underneath.
4. Open the classic Sound control panel: press Win+R, type `mmsys.cpl`, press
   Enter. The **Playback** and **Recording** tabs should each list the same 8
   cable devices with a green or grey status, not "Not present" or
   "Disabled".
5. In `mmsys.cpl`, double-click **Speakers / Audio Program Cable 1**, open the
   **Advanced** tab, and check the default format reads **2 channel, 32 bit,
   48000 Hz (Studio Quality)**.

Write down what you see. If all the devices are called **Audio Program
Virtual Cables** instead of "Audio Program Cable 1, 2, 3...", note that as a
naming problem, then keep testing.

## Step 6: Play into a cable and record from it

This is the main test: sound played into a cable must come out of the same
cable's microphone.

### 6a. Listen through the cable

1. In `mmsys.cpl`, open the **Recording** tab, double-click **Microphone /
   Audio Program Cable 1**, and open the **Listen** tab.
2. Tick **Listen to this device**. Under **Playback through this device**,
   choose your real speakers or headphones. Click **OK**.
3. Start some music or a video in any app (for example a browser).
4. Send that app's sound into Cable 1:
   - **Windows 11:** Settings > System > Sound > **Volume mixer**. Next to the
     app, set **Output device** to **Speakers (Audio Program Cable 1)**.
   - **Windows 10:** Settings > System > Sound > **App volume and device
     preferences**. Set the app's **Output** to **Speakers (Audio Program
     Cable 1)**.
5. You should still hear the music, because it now travels app > Cable 1
   Speakers > Cable 1 Microphone > your real speakers. In the **Recording**
   tab, the green level bar next to Cable 1's Microphone should move.
6. Listen for about a minute. It should sound clean: no crackles, gaps,
   robotic sound, or change in speed or pitch. A small delay is normal.
7. Pause the music. The level bar must drop to nothing, and you must hear
   silence, not a repeating snippet.

Untick **Listen to this device** when you are done.

### 6b. Record from the cable

1. Open the **Sound Recorder** app (called **Voice Recorder** on Windows 10).
2. Choose the microphone **Audio Program Cable 1**. In Windows 11 Sound
   Recorder this is the drop-down at the bottom-left. In Windows 10, make
   Cable 1's Microphone the default recording device first: in `mmsys.cpl` >
   **Recording**, right-click it and choose **Set as Default Device**.
3. Keep the music playing into **Speakers (Audio Program Cable 1)** as in 6a.
4. Record about 30 seconds, stop, and play the recording back through your
   real speakers. It should sound like the original.

### 6c. Check the other cables

1. Send the music to **Speakers (Audio Program Cable 2)** instead. In
   `mmsys.cpl` > **Recording**, only **Cable 2's** Microphone level bar
   should move, not Cable 1's.
2. Repeat for the last cable (Cable 8). You do not need to test every cable in
   between, but it is a bonus if you do.

## Step 7: Change the number of cables

Close every app that uses the cables (including Sound Recorder), then run:

```powershell
.\scripts\Set-CableCount.ps1 -Count 4
```

Open Sound settings again. There should now be exactly 4 cable Speakers and 4
cable Microphones. Then try the maximum and go back to the default:

```powershell
.\scripts\Set-CableCount.ps1 -Count 16
.\scripts\Set-CableCount.ps1 -Count 8
```

If a script says the device "will restart at the next reboot", an app was
still using a cable. Close it and run the command again, or restart the PC.

## Step 8: Check CPU use

1. Play music into Cable 1, with "Listen to this device" on (as in 6a).
2. Open **Task Manager** (Ctrl+Shift+Esc), go to **Details**, and sort by
   **CPU**.
3. The driver runs inside **System** and Windows' audio engine, which shows as
   **audiodg.exe**. Watch them for a few minutes. Together they should stay
   low (around 0 to 2 percent on a typical PC; a little more in a VM).
4. Stop the music. With nothing playing, both should drop back to about 0.

Write down the numbers you see while playing and while idle.

## Step 9: Watch for crashes

A bug in a driver can crash the whole computer with a blue (or black) screen,
known as a BSOD. During the whole test, note any crash, freeze or unexpected
restart. After testing:

1. Look in `C:\Windows\Minidump`. Any `.dmp` file with today's date means a
   crash happened.
2. Open **Event Viewer** (right-click Start > Event Viewer), go to **Windows
   Logs > System**, click **Filter Current Log**, and enter `41,1001` in the
   event ID box. Event 41 means the PC restarted unexpectedly; event 1001
   ("BugCheck") records a crash and its stop code.

To stress the driver a little more, repeat these a few times: start and stop
music; change the cable count; uninstall and reinstall (Step 10, then Step 4);
and on a physical PC, put it to sleep and wake it up while music is playing
into a cable.

**Optional, for experienced testers:** Windows' built-in Driver Verifier
checks the driver much more strictly. Only do this in a VM with a snapshot:

```powershell
verifier /standard /driver AudioProgramCable.sys
```

Restart, repeat Steps 5 to 7, then turn it off with `verifier /reset` and
restart. If the VM crashes during startup, restore the snapshot.

## Step 10: Uninstall and clean up

Close apps that use the cables, then in the administrator PowerShell:

```powershell
.\scripts\Uninstall-Driver.ps1 -RemoveCertificate
.\scripts\Enable-TestSigning.ps1 -Disable
```

Restart. The "Test Mode" text should be gone, and Sound settings should no
longer list any Audio Program devices. On a physical PC, turn Secure Boot back
on in the firmware setup. In a VM, you can simply restore the "before driver"
snapshot instead.

## If something goes wrong

| What you see | What it usually means | What to do |
| --- | --- | --- |
| "running scripts is disabled on this system" | PowerShell's script policy | Run the `Set-ExecutionPolicy -Scope Process ...` line again in this window. |
| Device Manager shows a yellow triangle, code 52 | Windows does not trust the signature | Check "Test Mode" is shown on the desktop and Secure Boot is off, then reinstall. |
| Yellow triangle, code 10 | The driver failed to start | Note it. Send `C:\Windows\INF\setupapi.dev.log` and the `Get-CableStatus.ps1` output. |
| Device is OK but no cables in Sound settings | Endpoints were not created | Restart the PC once. If still missing, send the `Get-CableStatus.ps1` output. |
| All cables are named "Audio Program Virtual Cables" | Per-cable names were not applied | Note it as a naming problem; the cables may still work. |
| Your speakers went silent after install | Windows made a cable the default output | Pick your real speakers in Settings > System > Sound. |
| Install fails with "Memory integrity" or "driver blocked" | Core isolation blocked the test driver | Note it. You can turn off Windows Security > Device security > Core isolation > Memory integrity to continue. |

## What to send back

- The filled-in checklist below.
- The output of `.\scripts\Get-CableStatus.ps1`.
- Screenshots of Settings > System > Sound (Output and Input lists).
- Your Windows version (Win+R, `winver`) and whether it was a PC or a VM.
- If anything failed: `C:\Windows\INF\setupapi.dev.log` and any files in
  `C:\Windows\Minidump`.

## Pass/fail checklist

| # | Check | Pass | Fail | Notes |
| --- | --- | --- | --- | --- |
| 1 | "Test Mode" shows after enabling test signing and restarting | [ ] | [ ] | |
| 2 | `Install-Driver.ps1` finishes with Status OK | [ ] | [ ] | |
| 3 | Device Manager shows "Audio Program Virtual Cables" with no warning | [ ] | [ ] | |
| 4 | **Devices appear:** 8 cable Speakers and 8 cable Microphones in Sound settings | [ ] | [ ] | |
| 5 | Cables are named "Audio Program Cable 1" to "Audio Program Cable 8" | [ ] | [ ] | |
| 6 | Default format is 2 channel, 32 bit, 48000 Hz | [ ] | [ ] | |
| 7 | **Another app plays into a cable and you hear it back** through "Listen to this device" (6a) | [ ] | [ ] | |
| 8 | **Recording from the cable** with Sound Recorder sounds like the original (6b) | [ ] | [ ] | |
| 9 | Silence when playback stops; no repeating snippet | [ ] | [ ] | |
| 10 | Cables are separate: playing into Cable 2 does not show on Cable 1 | [ ] | [ ] | |
| 11 | `Set-CableCount.ps1 -Count 4`, then `16`, then `8` gives that many cables | [ ] | [ ] | |
| 12 | **CPU use:** System + audiodg.exe stay low while playing, about 0 when idle | [ ] | [ ] | playing: ___ % idle: ___ % |
| 13 | **No crash or BSOD** during testing; no new files in `C:\Windows\Minidump`; no event 41/1001 | [ ] | [ ] | |
| 14 | `Uninstall-Driver.ps1` removes all cable devices | [ ] | [ ] | |

The driver passes when items 2, 3, 4, 7, 8, 9, 10, 11, 12 and 13 all pass.
Item 5 failing on its own is a naming bug, not a blocker.
