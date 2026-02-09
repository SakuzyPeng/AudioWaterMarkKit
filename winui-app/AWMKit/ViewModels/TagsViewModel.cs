using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System.Collections.ObjectModel;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// View model for the Tags management page.
/// </summary>
public sealed partial class TagsViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _isLoading;

    [ObservableProperty]
    private TagMapping? _selectedMapping;

    [ObservableProperty]
    private string _newIdentity = string.Empty;

    [ObservableProperty]
    private string _newTag = string.Empty;

    [ObservableProperty]
    private string _newDisplayName = string.Empty;

    [ObservableProperty]
    private string? _errorMessage;

    public ObservableCollection<TagMapping> TagMappings { get; } = new();

    /// <summary>
    /// Loads all tag mappings from the database.
    /// </summary>
    [RelayCommand]
    public async Task LoadMappingsAsync()
    {
        IsLoading = true;
        ErrorMessage = null;

        await Task.Run(async () =>
        {
            var mappings = await AppViewModel.Instance.TagStore.ListAllAsync();

            App.Current.MainWindow?.DispatcherQueue.TryEnqueue(() =>
            {
                TagMappings.Clear();
                foreach (var mapping in mappings)
                {
                    TagMappings.Add(mapping);
                }
            });

            IsLoading = false;
        });
    }

    /// <summary>
    /// Generates a random tag suggestion.
    /// </summary>
    [RelayCommand]
    private void SuggestTag()
    {
        try
        {
            // Use NewIdentity as username, or generate random if empty
            string username = string.IsNullOrEmpty(NewIdentity) ? "user" : NewIdentity;
            var (tag, error) = AwmBridge.SuggestTag(username);
            if (error == AwmError.Ok && tag is not null)
            {
                NewTag = tag;
                ErrorMessage = null;
            }
            else
            {
                ErrorMessage = $"Failed to generate tag: {error}";
            }
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Failed to generate tag: {ex.Message}";
        }
    }

    /// <summary>
    /// Saves a new or updated tag mapping.
    /// </summary>
    [RelayCommand]
    private async Task SaveMappingAsync()
    {
        if (string.IsNullOrEmpty(NewIdentity) || string.IsNullOrEmpty(NewTag))
        {
            ErrorMessage = "Identity and Tag are required";
            return;
        }

        IsLoading = true;
        ErrorMessage = null;

        await Task.Run(async () =>
        {
            var success = await AppViewModel.Instance.TagStore.SaveAsync(
                NewIdentity,
                NewTag,
                string.IsNullOrEmpty(NewDisplayName) ? null : NewDisplayName);

            if (success)
            {
                // Clear form
                App.Current.MainWindow?.DispatcherQueue.TryEnqueue(() =>
                {
                    NewIdentity = string.Empty;
                    NewTag = string.Empty;
                    NewDisplayName = string.Empty;
                });

                // Reload list
                await LoadMappingsAsync();

                // Refresh app stats
                await AppViewModel.Instance.RefreshStatsAsync();
            }
            else
            {
                ErrorMessage = "Failed to save mapping (duplicate tag?)";
            }

            IsLoading = false;
        });
    }

    /// <summary>
    /// Deletes a tag mapping by identity.
    /// </summary>
    [RelayCommand]
    public async Task DeleteMappingAsync(TagMapping? mapping)
    {
        if (mapping is null)
        {
            return;
        }

        IsLoading = true;
        ErrorMessage = null;

        await Task.Run(async () =>
        {
            var success = await AppViewModel.Instance.TagStore.DeleteByIdentityAsync(mapping.Identity);

            if (success)
            {
                // Also delete associated evidence records
                await AppViewModel.Instance.EvidenceStore.DeleteByTagAsync(mapping.Tag);

                // Reload list
                await LoadMappingsAsync();

                // Refresh app stats
                await AppViewModel.Instance.RefreshStatsAsync();
            }
            else
            {
                ErrorMessage = "Failed to delete mapping";
            }

            IsLoading = false;
        });
    }

    /// <summary>
    /// Deletes all evidence records for a specific tag.
    /// </summary>
    [RelayCommand]
    public async Task DeleteEvidenceAsync(TagMapping? mapping)
    {
        if (mapping is null)
        {
            return;
        }

        IsLoading = true;
        ErrorMessage = null;

        await Task.Run(async () =>
        {
            var count = await AppViewModel.Instance.EvidenceStore.DeleteByTagAsync(mapping.Tag);

            ErrorMessage = count > 0 ? $"Deleted {count} evidence record(s)" : "No evidence found";

            // Refresh app stats
            await AppViewModel.Instance.RefreshStatsAsync();

            IsLoading = false;
        });
    }

    /// <summary>
    /// Checks if a mapping can be saved.
    /// </summary>
    public bool CanSave => !string.IsNullOrEmpty(NewIdentity) && !string.IsNullOrEmpty(NewTag) && !IsLoading;
}
