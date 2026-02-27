using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Threading.Tasks;
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace AWMKit.Pages;

/// <summary>
/// Key management page.
/// </summary>
public sealed partial class KeyPage : Page
{
    public KeyViewModel ViewModel { get; } = new();

    public KeyPage()
    {
        InitializeComponent();
        ViewModel.PropertyChanged += ViewModelOnPropertyChanged;
    }

    private async void Page_Loaded(object sender, RoutedEventArgs e)
    {
        await ViewModel.InitializeAsync();
    }

    private async void GenerateKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var error = await ViewModel.GenerateKeyAsync();
        if (error == Native.AwmError.KeyAlreadyExists)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("槽位已有密钥", "Slot already has key"),
                AppStrings.Pick("当前槽位已存在密钥，已阻止覆盖。\n如需替换，请先删除该槽位密钥后再生成。", "Current slot already has a key and overwrite is blocked.\nDelete this slot key before generating a new one."));
            return;
        }

        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("生成失败", "Generate failed"),
                $"{AppStrings.Pick("密钥生成失败", "Key generation failed")}: {error}");
        }
    }

    private async void EditLabelButton_Click(object sender, RoutedEventArgs e)
    {
        var activeSlot = ViewModel.ActiveKeySlot;
        var activeSummary = ViewModel.ActiveKeySummary;
        var editor = new TextBox
        {
            PlaceholderText = AppStrings.Pick("输入新标签（留空表示清除）", "Enter new label (leave empty to clear)"),
            Text = activeSummary?.Label ?? string.Empty
        };

        var content = new StackPanel { Spacing = 8 };
        content.Children.Add(new TextBlock { Text = AppStrings.Pick($"当前激活槽位：{activeSlot}", $"Active slot: {activeSlot}") });
        content.Children.Add(new TextBlock { Text = $"Key ID: {activeSummary?.KeyId ?? AppStrings.Pick("未配置", "Not configured")}" });
        content.Children.Add(new TextBlock { Text = AppStrings.Pick($"当前标签：{(string.IsNullOrWhiteSpace(activeSummary?.Label) ? "未设置" : activeSummary!.Label)}", $"Current label: {(string.IsNullOrWhiteSpace(activeSummary?.Label) ? "not set" : activeSummary!.Label)}") });
        content.Children.Add(editor);

        var dialog = new ContentDialog
        {
            Title = AppStrings.Pick("编辑槽位标签", "Edit slot label"),
            Content = content,
            PrimaryButtonText = AppStrings.Pick("保存", "Save"),
            CloseButtonText = AppStrings.Pick("取消", "Cancel"),
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = XamlRoot
        };

        var result = await dialog.ShowAsync();
        if (result != ContentDialogResult.Primary)
        {
            return;
        }

        var error = await ViewModel.EditActiveSlotLabelAsync(editor.Text);
        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("编辑失败", "Edit failed"),
                $"{AppStrings.Pick("密钥标签更新失败", "Key label update failed")}: {error}");
        }
    }

    private async void ImportKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var file = await PickKeyImportFileAsync();
        if (file is null)
        {
            return;
        }

        byte[] keyBytes;
        try
        {
            keyBytes = await File.ReadAllBytesAsync(file.Path);
        }
        catch (Exception ex)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("导入失败", "Import failed"),
                $"{AppStrings.Pick("读取密钥文件失败", "Failed to read key file")}: {ex.Message}");
            return;
        }

        if (keyBytes.Length != 32)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("导入失败", "Import failed"),
                $"{AppStrings.Pick("密钥文件必须为 32 字节", "Key file must be exactly 32 bytes")}: {keyBytes.Length}");
            return;
        }

        var error = await ViewModel.ImportKeyBytesAsync(keyBytes);
        if (error == Native.AwmError.KeyAlreadyExists)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("槽位已有密钥", "Slot already has key"),
                AppStrings.Pick("当前槽位已存在密钥，已阻止覆盖。\n如需替换，请先删除该槽位密钥后再导入。", "Current slot already has a key and overwrite is blocked.\nDelete this slot key before importing."));
            return;
        }

        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("导入失败", "Import failed"),
                $"{AppStrings.Pick("密钥导入失败", "Key import failed")}: {error}");
        }
    }

    private async void ImportHexButton_Click(object sender, RoutedEventArgs e)
    {
        var input = new TextBox
        {
            PlaceholderText = AppStrings.Pick("请输入 64 位十六进制字符（可带 0x 前缀）", "Enter 64 hex characters (0x prefix allowed)"),
            TextWrapping = TextWrapping.Wrap,
            AcceptsReturn = true,
            MinHeight = 110
        };

        var content = new StackPanel { Spacing = 8 };
        content.Children.Add(new TextBlock { Text = AppStrings.Pick($"目标槽位：{ViewModel.SelectedSlot}", $"Target slot: {ViewModel.SelectedSlot}") });
        content.Children.Add(input);

        var dialog = new ContentDialog
        {
            Title = AppStrings.Pick("Hex 密钥导入", "Hex key import"),
            Content = content,
            PrimaryButtonText = AppStrings.Pick("导入", "Import"),
            CloseButtonText = AppStrings.Pick("取消", "Cancel"),
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = XamlRoot
        };

        var result = await dialog.ShowAsync();
        if (result != ContentDialogResult.Primary)
        {
            return;
        }

        var error = await ViewModel.ImportHexAsync(input.Text);
        if (error == Native.AwmError.KeyAlreadyExists)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("槽位已有密钥", "Slot already has key"),
                AppStrings.Pick("当前槽位已存在密钥，已阻止覆盖。\n如需替换，请先删除该槽位密钥后再导入。", "Current slot already has a key and overwrite is blocked.\nDelete this slot key before importing."));
            return;
        }

        if (error == Native.AwmError.InvalidMessageLength)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("Hex 导入失败", "Hex import failed"),
                AppStrings.Pick("请输入 64 位十六进制字符（可带 0x 前缀）。", "Enter 64 hex characters (0x prefix allowed)."));
            return;
        }

        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("Hex 导入失败", "Hex import failed"),
                $"{AppStrings.Pick("密钥导入失败", "Key import failed")}: {error}");
        }
    }

    private async void ExportKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var (key, error) = await ViewModel.ExportKeyBytesAsync();
        if (error != Native.AwmError.Ok || key is null)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("导出失败", "Export failed"),
                $"{AppStrings.Pick("读取槽位密钥失败", "Failed to load slot key")}: {error}");
            return;
        }

        var file = await PickKeyExportFileAsync(ViewModel.SelectedSlot);
        if (file is null)
        {
            return;
        }

        try
        {
            await FileIO.WriteBytesAsync(file, key);
            await ViewModel.MarkExportSuccessAsync();
        }
        catch (Exception ex)
        {
            await ShowMessageDialogAsync(
                AppStrings.Pick("导出失败", "Export failed"),
                $"{AppStrings.Pick("写入密钥文件失败", "Failed to write key file")}: {ex.Message}");
        }
    }

    private async void DeleteKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var slot = ViewModel.SelectedSlot;
        var instruction = new TextBlock
        {
            Text = AppStrings.Pick(
                $"此操作不可恢复。请输入槽位号 {slot} 以确认删除该槽位密钥。",
                $"This action cannot be undone. Enter slot number {slot} to confirm deleting this slot key."),
            TextWrapping = TextWrapping.Wrap
        };

        var inputBox = new TextBox
        {
            PlaceholderText = AppStrings.Pick($"输入槽位号 {slot}", $"Enter slot number {slot}")
        };

        var hint = new TextBlock
        {
            Text = AppStrings.Pick("输入不匹配时无法确认删除", "Delete confirmation disabled when input does not match"),
            Foreground = GetBrush("TextFillColorSecondaryBrush")
        };

        var content = new StackPanel
        {
            Spacing = 10,
            Children = { instruction, inputBox, hint }
        };

        var dialog = new ContentDialog
        {
            Title = AppStrings.Pick("删除密钥", "Delete key"),
            Content = content,
            PrimaryButtonText = AppStrings.Pick("删除", "Delete"),
            CloseButtonText = AppStrings.Pick("取消", "Cancel"),
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = XamlRoot,
            IsPrimaryButtonEnabled = false
        };

        inputBox.TextChanged += (_, _) =>
        {
            dialog.IsPrimaryButtonEnabled = IsDeleteSlotInputValid(inputBox.Text, slot);
        };

        var result = await dialog.ShowAsync();
        if (result == ContentDialogResult.Primary)
        {
            await ViewModel.DeleteKeyAsync();
        }
    }

    private async void ApplySlotButton_Click(object sender, RoutedEventArgs e)
    {
        await ViewModel.SaveSelectedSlotAsync();
    }

    private async void RefreshButton_Click(object sender, RoutedEventArgs e)
    {
        await ViewModel.RefreshStatusAsync();
    }

    private void ViewModelOnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        _ = DispatcherQueue.TryEnqueue(() =>
        {
            if (e.PropertyName is nameof(KeyViewModel.IsBusy)
                or nameof(KeyViewModel.KeyAvailable)
                or nameof(KeyViewModel.SelectedSlotHasKey)
                or nameof(KeyViewModel.KeyStatusText)
                or nameof(KeyViewModel.KeySourceLabel)
                or nameof(KeyViewModel.KeyStatusSeverity)
                or nameof(KeyViewModel.KeyStatusMessage)
                or nameof(KeyViewModel.CanOperate)
                or nameof(KeyViewModel.CanGenerateKey)
                or nameof(KeyViewModel.CanImportKey)
                or nameof(KeyViewModel.CanExportKey)
                or nameof(KeyViewModel.GenerateKeyTooltip)
                or nameof(KeyViewModel.ImportActionBrush)
                or nameof(KeyViewModel.ImportHexActionBrush)
                or nameof(KeyViewModel.ExportActionBrush)
                or nameof(KeyViewModel.ImportFileActionText)
                or nameof(KeyViewModel.ImportHexActionText)
                or nameof(KeyViewModel.ExportFileActionText))
            {
                Bindings.Update();
            }
        });
    }

    private async Task ShowMessageDialogAsync(string title, string content)
    {
        var dialog = new ContentDialog
        {
            Title = title,
            Content = content,
            CloseButtonText = AppStrings.Pick("确定", "OK"),
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = XamlRoot
        };

        await dialog.ShowAsync();
    }

    private static bool IsDeleteSlotInputValid(string input, int expectedSlot)
    {
        return int.TryParse(input.Trim(), out var parsed) && parsed == expectedSlot;
    }

    private static Brush GetBrush(string resourceKey)
    {
        if (Application.Current.Resources.TryGetValue(resourceKey, out var value) && value is Brush brush)
        {
            return brush;
        }

        if (Application.Current.Resources.TryGetValue("TextFillColorSecondaryBrush", out var fallback)
            && fallback is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }

    private async Task<StorageFile?> PickKeyImportFileAsync()
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".bin");
        picker.FileTypeFilter.Add("*");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);
        return await picker.PickSingleFileAsync();
    }

    private async Task<StorageFile?> PickKeyExportFileAsync(int slot)
    {
        var picker = new FileSavePicker
        {
            SuggestedFileName = $"awmkit-key-slot-{slot}"
        };
        picker.FileTypeChoices.Add("Binary key (.bin)", new List<string> { ".bin" });

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);
        return await picker.PickSaveFileAsync();
    }


}
