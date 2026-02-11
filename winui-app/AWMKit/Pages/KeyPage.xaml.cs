using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System.ComponentModel;
using System.Threading.Tasks;

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
                L("槽位已有密钥", "Slot already has key"),
                L("当前槽位已存在密钥，已阻止覆盖。\n如需替换，请先删除该槽位密钥后再生成。", "Current slot already has a key and overwrite is blocked.\nDelete this slot key before generating a new one."));
            return;
        }

        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                L("生成失败", "Generate failed"),
                $"{L("密钥生成失败", "Key generation failed")}: {error}");
        }
    }

    private async void EditLabelButton_Click(object sender, RoutedEventArgs e)
    {
        var activeSlot = ViewModel.ActiveKeySlot;
        var activeSummary = ViewModel.ActiveKeySummary;
        var editor = new TextBox
        {
            PlaceholderText = L("输入新标签（留空表示清除）", "Enter new label (leave empty to clear)"),
            Text = activeSummary?.Label ?? string.Empty
        };

        var content = new StackPanel { Spacing = 8 };
        content.Children.Add(new TextBlock { Text = L($"当前激活槽位：{activeSlot}", $"Active slot: {activeSlot}") });
        content.Children.Add(new TextBlock { Text = $"Key ID: {activeSummary?.KeyId ?? L("未配置", "Not configured")}" });
        content.Children.Add(new TextBlock { Text = L($"当前标签：{(string.IsNullOrWhiteSpace(activeSummary?.Label) ? "未设置" : activeSummary!.Label)}", $"Current label: {(string.IsNullOrWhiteSpace(activeSummary?.Label) ? "not set" : activeSummary!.Label)}") });
        content.Children.Add(editor);

        var dialog = new ContentDialog
        {
            Title = L("编辑槽位标签", "Edit slot label"),
            Content = content,
            PrimaryButtonText = L("保存", "Save"),
            CloseButtonText = L("取消", "Cancel"),
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
                L("编辑失败", "Edit failed"),
                $"{L("密钥标签更新失败", "Key label update failed")}: {error}");
        }
    }

    private async void DeleteKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var slot = ViewModel.SelectedSlot;
        var instruction = new TextBlock
        {
            Text = L(
                $"此操作不可恢复。请输入槽位号 {slot} 以确认删除该槽位密钥。",
                $"This action cannot be undone. Enter slot number {slot} to confirm deleting this slot key."),
            TextWrapping = TextWrapping.Wrap
        };

        var inputBox = new TextBox
        {
            PlaceholderText = L($"输入槽位号 {slot}", $"Enter slot number {slot}")
        };

        var hint = new TextBlock
        {
            Text = L("输入不匹配时无法确认删除", "Delete confirmation disabled when input does not match"),
            Foreground = GetBrush("TextFillColorSecondaryBrush")
        };

        var content = new StackPanel
        {
            Spacing = 10,
            Children = { instruction, inputBox, hint }
        };

        var dialog = new ContentDialog
        {
            Title = L("删除密钥", "Delete key"),
            Content = content,
            PrimaryButtonText = L("删除", "Delete"),
            CloseButtonText = L("取消", "Cancel"),
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
                or nameof(KeyViewModel.GenerateKeyTooltip))
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
            CloseButtonText = L("确定", "OK"),
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

    private static string L(string zh, string en) => AppViewModel.Instance.IsEnglishLanguage ? en : zh;
}
