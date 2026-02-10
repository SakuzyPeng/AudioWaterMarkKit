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
        await ViewModel.GenerateKeyAsync();
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
        await ViewModel.InitializeAsync();
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
}
