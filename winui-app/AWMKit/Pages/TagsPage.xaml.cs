using AWMKit.Models;
using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System.Threading.Tasks;

namespace AWMKit.Pages;

/// <summary>
/// Database query page for mapping/evidence management.
/// </summary>
public sealed partial class TagsPage : Page
{
    public TagsViewModel ViewModel { get; }

    public TagsPage()
    {
        InitializeComponent();
        ViewModel = new TagsViewModel();
    }

    private async void Page_Loaded(object sender, RoutedEventArgs e)
    {
        await ViewModel.InitializeAsync();
    }

    private void ErrorInfoBar_CloseButtonClick(InfoBar sender, object args)
    {
        ViewModel.ClearErrorMessage();
    }

    private void InfoInfoBar_CloseButtonClick(InfoBar sender, object args)
    {
        ViewModel.ClearInfoMessage();
    }

    private async void AddMappingButton_Click(object sender, RoutedEventArgs e)
    {
        var usernameBox = new TextBox
        {
            PlaceholderText = "例如: user_001"
        };
        var previewValue = new TextBlock
        {
            Text = "-",
            FontFamily = new Microsoft.UI.Xaml.Media.FontFamily("Consolas"),
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold
        };
        var hintText = new TextBlock
        {
            Text = string.Empty,
            Foreground = GetBrush("TextFillColorSecondaryBrush")
        };

        var content = new StackPanel
        {
            Spacing = 10,
            Children =
            {
                new TextBlock { Text = "用户名" },
                usernameBox,
                new TextBlock { Text = "Tag 预览" },
                previewValue,
                hintText
            }
        };

        var dialog = new ContentDialog
        {
            Title = "添加标签映射",
            PrimaryButtonText = "保存",
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Primary,
            Content = content,
            XamlRoot = XamlRoot,
            IsPrimaryButtonEnabled = false
        };

        void RefreshPreview()
        {
            var preview = ViewModel.ResolveTagPreview(usernameBox.Text, out var reusedExisting);
            previewValue.Text = preview;
            hintText.Text = reusedExisting ? "已存在映射，自动复用" : "将新增该用户名映射";
            dialog.IsPrimaryButtonEnabled = !string.IsNullOrWhiteSpace(usernameBox.Text) && preview != "-";
        }

        usernameBox.TextChanged += (_, _) => RefreshPreview();
        RefreshPreview();

        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }

        await ViewModel.AddMappingFromUsernameAsync(usernameBox.Text);
    }

    private void EnterDeleteMappingsModeButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.EnterMappingsDeleteMode();
    }

    private void EnterDeleteEvidenceModeButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.EnterEvidenceDeleteMode();
    }

    private void ExitDeleteModeButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.ExitDeleteMode();
    }

    private void SelectAllButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.SelectAllInCurrentMode();
    }

    private void ClearSelectionButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.ClearSelectionInCurrentMode();
    }

    private async void ExecuteDeleteButton_Click(object sender, RoutedEventArgs e)
    {
        var selectedCount = ViewModel.GetCurrentSelectionCount();
        if (selectedCount <= 0)
        {
            ViewModel.ExitDeleteMode();
            return;
        }

        var confirmed = await ShowDeleteConfirmDialogAsync(selectedCount, ViewModel.DeleteTargetLabel);
        if (!confirmed)
        {
            return;
        }

        await ViewModel.ExecuteDeleteAsync();
    }

    private static bool IsDeleteInputValid(string input, int expectedCount)
    {
        return int.TryParse(input.Trim(), out var parsed) && parsed == expectedCount;
    }

    private async Task<bool> ShowDeleteConfirmDialogAsync(int expectedCount, string noun)
    {
        var instruction = new TextBlock
        {
            Text = $"请输入数字 {expectedCount} 以确认删除 {expectedCount} 条{noun}",
            TextWrapping = TextWrapping.Wrap
        };

        var inputBox = new TextBox
        {
            PlaceholderText = $"输入 {expectedCount}"
        };

        var hint = new TextBlock
        {
            Text = "数量不匹配时无法确认",
            Foreground = GetBrush("TextFillColorSecondaryBrush")
        };

        var content = new StackPanel
        {
            Spacing = 10,
            Children = { instruction, inputBox, hint }
        };

        var dialog = new ContentDialog
        {
            Title = "确认删除",
            PrimaryButtonText = "确认删除",
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Close,
            Content = content,
            XamlRoot = XamlRoot,
            IsPrimaryButtonEnabled = false
        };

        inputBox.TextChanged += (_, _) =>
        {
            dialog.IsPrimaryButtonEnabled = IsDeleteInputValid(inputBox.Text, expectedCount);
        };

        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }

    private void MappingsListView_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is TagMapping mapping)
        {
            ViewModel.ToggleMappingSelection(mapping);
        }
    }

    private void EvidenceListView_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is EvidenceRecord record)
        {
            ViewModel.ToggleEvidenceSelection(record);
        }
    }

    private static Brush GetBrush(string resourceKey)
    {
        if (Application.Current.Resources.TryGetValue(resourceKey, out var value) && value is Brush brush)
        {
            return brush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Gray);
    }
}
