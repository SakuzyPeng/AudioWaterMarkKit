# Assets Directory

Place application assets here:

- `AppIcon.png` - Main application icon (16x16 or 32x32 for title bar)
- `AppLogo.png` - Application logo (for splash screen, etc.)
- Other image assets

## Required Assets

- **AppIcon.png**: Referenced in `MainWindow.xaml` title bar. Should be a 16x16 or 32x32 PNG icon.

## Temporary Workaround

If `AppIcon.png` is missing, the title bar will show a broken image. Either:
1. Create a simple icon file
2. Remove the `<Image>` element from `MainWindow.xaml` line 27
