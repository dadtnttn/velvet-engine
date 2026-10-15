param([string]$Title = "Velvet Editor", [string]$Out)
# Captures ONE window: the top-level window whose title holds $Title. If it is minimized it is
# shown without activation at the bottom of the z-order, captured, and minimized again.
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class Grab {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  public static IntPtr Find(string title) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, p) => {
      var sb = new StringBuilder(512);
      GetWindowText(h, sb, 512);
      if (IsWindowVisible(h) && sb.ToString().Contains(title)) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@
[void][Grab]::SetProcessDPIAware()
$h = [Grab]::Find($Title)
if ($h -eq [IntPtr]::Zero) { Write-Output "NO_WINDOW"; exit 2 }
$wasMin = [Grab]::IsIconic($h)
if ($wasMin) {
  [void][Grab]::ShowWindow($h, 4)                                  # SW_SHOWNOACTIVATE
  [void][Grab]::SetWindowPos($h, [IntPtr]1, 0, 0, 0, 0, 0x0013)    # HWND_BOTTOM, keep size, no activate
  for ($i = 0; $i -lt 30 -and [Grab]::IsIconic($h); $i++) { Start-Sleep -Milliseconds 100 }
  Start-Sleep -Milliseconds 1200                                   # let it repaint
}
$r = New-Object Grab+RECT
[void][Grab]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [Grab]::PrintWindow($h, $hdc, 2)                             # PW_RENDERFULLCONTENT
$g.ReleaseHdc($hdc); $g.Dispose()
$c = New-Object Grab+RECT
[void][Grab]::GetClientRect($h, [ref]$c)
$pt = New-Object Grab+POINT
[void][Grab]::ClientToScreen($h, [ref]$pt)
$crop = New-Object System.Drawing.Rectangle ($pt.X - $r.L), ($pt.Y - $r.T), $c.R, $c.B
if ($c.R -gt 0 -and $c.B -gt 0) {
  $client = $bmp.Clone($crop, $bmp.PixelFormat)
  $client.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
  $client.Dispose()
}
$bmp.Dispose()
if ($wasMin) { [void][Grab]::ShowWindow($h, 7) }                   # SW_SHOWMINNOACTIVE
Write-Output ("ok={0} ventana={1}x{2} cliente={3}x{4} estaba_minimizada={5}" -f $ok, $w, $hh, $c.R, $c.B, $wasMin)
