using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.ComponentModel;
using System.Threading.Tasks;

namespace AWMKit.Pages;

/// <summary>
/// Key management page.
/// </summary>
public sealed partial class KeyPage : Page
{
    public KeyViewModel ViewModel { get; } = new();

    public bool IsNotBusy => !ViewModel.IsBusy;
    public bool CanGenerateKey => IsNotBusy && !ViewModel.SelectedSlotHasKey;
    public string GenerateKeyTooltip => ViewModel.SelectedSlotHasKey
        ? "当前槽位已有密钥，已禁止覆盖。请先删除后再生成。"
        : "在当前槽位生成新密钥";

    public InfoBarSeverity KeyStatusSeverity => ViewModel.KeyAvailable ? InfoBarSeverity.Success : InfoBarSeverity.Warning;

    public string KeyStatusMessage => ViewModel.KeyAvailable
        ? "密钥已配置，可正常嵌入与检测。"
        : "未配置密钥。请先生成密钥后再执行嵌入/检测。";

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
                "槽位已有密钥",
                "当前槽位已存在密钥，已阻止覆盖。\n如需替换，请先删除该槽位密钥后再生成。");
            return;
        }

        if (error != Native.AwmError.Ok)
        {
            await ShowMessageDialogAsync(
                "生成失败",
                $"密钥生成失败：{error}");
        }
    }

    private async void EditLabelButton_Click(object sender, RoutedEventArgs e)
    {
        var activeSlot = ViewModel.ActiveKeySlot;
        var activeSummary = ViewModel.ActiveKeySummary;
        var editor = new TextBox
        {
            PlaceholderText = "输入新标签（留空表示清除）",
            Text = activeSummary?.Label ?? string.Empty
        };

        var content = new StackPanel { Spacing = 8 };
        content.Children.Add(new TextBlock { Text = $"当前激活槽位：{activeSlot}" });
        content.Children.Add(new TextBlock { Text = $"Key ID：{activeSummary?.KeyId ?? "未配置"}" });
        content.Children.Add(new TextBlock { Text = $"当前标签：{(string.IsNullOrWhiteSpace(activeSummary?.Label) ? "未设置" : activeSummary!.Label)}" });
        content.Children.Add(editor);

        var dialog = new ContentDialog
        {
            Title = "编辑槽位标签",
            Content = content,
            PrimaryButtonText = "保存",
            CloseButtonText = "取消",
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
                "编辑失败",
                $"密钥标签更新失败：{error}");
        }
    }

    private async void DeleteKeyButton_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new ContentDialog
        {
            Title = "删除密钥",
            Content = "删除后将无法继续嵌入/检测，是否确认删除？",
            PrimaryButtonText = "删除",
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = XamlRoot
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
            if (e.PropertyName == nameof(KeyViewModel.IsBusy))
            {
                Bindings.Update();
                return;
            }

            if (e.PropertyName is nameof(KeyViewModel.KeyAvailable)
                or nameof(KeyViewModel.SelectedSlotHasKey)
                or nameof(KeyViewModel.KeyStatusText)
                or nameof(KeyViewModel.KeySourceLabel))
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
            CloseButtonText = "确定",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = XamlRoot
        };

        await dialog.ShowAsync();
    }
}
