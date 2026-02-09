using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

public enum TagsDeleteMode
{
    None,
    Mappings,
    Evidence
}

/// <summary>
/// View model for the WinUI database query page (tag mappings + evidence).
/// </summary>
public sealed class TagsViewModel : ObservableObject
{
    private readonly HashSet<string> _selectedMappingUsernames = new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<long> _selectedEvidenceIds = new();
    private readonly List<TagMapping> _allMappings = [];
    private readonly List<EvidenceRecord> _allEvidence = [];
    private int? _scopeBeforeDeleteMode;

    private bool _isLoading;
    public bool IsLoading
    {
        get => _isLoading;
        private set
        {
            if (SetProperty(ref _isLoading, value))
            {
                OnPropertyChanged(nameof(CanOpenAddDialog));
                // Refresh delete-mode command states when loading transitions complete.
                NotifyDeletePresentationChanged();
            }
        }
    }

    private string _searchText = string.Empty;
    public string SearchText
    {
        get => _searchText;
        set
        {
            if (SetProperty(ref _searchText, value))
            {
                RefreshFilteredCollections();
            }
        }
    }

    private int _searchScopeIndex;
    public int SearchScopeIndex
    {
        get => _searchScopeIndex;
        set
        {
            var normalized = value switch
            {
                < 0 => 0,
                > 2 => 2,
                _ => value
            };
            if (SetProperty(ref _searchScopeIndex, normalized))
            {
                RefreshFilteredCollections();
                OnPropertyChanged(nameof(ShowMappingsPanel));
                OnPropertyChanged(nameof(ShowEvidencePanel));
            }
        }
    }

    private TagsDeleteMode _deleteMode = TagsDeleteMode.None;
    public TagsDeleteMode DeleteMode
    {
        get => _deleteMode;
        private set
        {
            if (SetProperty(ref _deleteMode, value))
            {
                NotifyDeletePresentationChanged();
            }
        }
    }

    private string? _errorMessage;
    public string? ErrorMessage
    {
        get => _errorMessage;
        private set
        {
            if (SetProperty(ref _errorMessage, value))
            {
                OnPropertyChanged(nameof(HasErrorMessage));
            }
        }
    }

    private string? _infoMessage;
    public string? InfoMessage
    {
        get => _infoMessage;
        private set
        {
            if (SetProperty(ref _infoMessage, value))
            {
                OnPropertyChanged(nameof(HasInfoMessage));
            }
        }
    }

    public bool HasErrorMessage => !string.IsNullOrWhiteSpace(ErrorMessage);
    public bool HasInfoMessage => !string.IsNullOrWhiteSpace(InfoMessage);

    public bool ShowMappingsPanel => SearchScopeIndex != 2;
    public bool ShowEvidencePanel => SearchScopeIndex != 1;

    public bool IsNormalMode => DeleteMode == TagsDeleteMode.None;
    public bool IsMappingsDeleteMode => DeleteMode == TagsDeleteMode.Mappings;
    public bool IsEvidenceDeleteMode => DeleteMode == TagsDeleteMode.Evidence;

    public bool CanOpenAddDialog => !IsLoading && IsNormalMode;
    public bool CanEnterMappingsDeleteMode => IsNormalMode && _allMappings.Count > 0 && !IsLoading;
    public bool CanEnterEvidenceDeleteMode => IsNormalMode && _allEvidence.Count > 0 && !IsLoading;
    public bool CanSelectAll => DeleteMode switch
    {
        TagsDeleteMode.Mappings => _allMappings.Count > 0,
        TagsDeleteMode.Evidence => _allEvidence.Count > 0,
        _ => false
    };
    public bool CanClearSelection => SelectedCount > 0;
    public bool CanExecuteDelete => DeleteMode != TagsDeleteMode.None && !IsLoading;

    public string DeleteTargetLabel => DeleteMode == TagsDeleteMode.Evidence ? "证据" : "标签";
    public int SelectedCount => DeleteMode switch
    {
        TagsDeleteMode.Mappings => _selectedMappingUsernames.Count,
        TagsDeleteMode.Evidence => _selectedEvidenceIds.Count,
        _ => 0
    };

    public int MappingTotalCount => _allMappings.Count;
    public int EvidenceTotalCount => _allEvidence.Count;
    public string MappingTotalBadgeText => $"{MappingTotalCount} 映射";
    public string EvidenceTotalBadgeText => $"{EvidenceTotalCount} 证据";
    public string MappingSummaryText => $"{FilteredMappings.Count}/{MappingTotalCount}";
    public string EvidenceSummaryText => $"{FilteredEvidence.Count}/{EvidenceTotalCount}";

    public bool ShowMappingEmptyHint => FilteredMappings.Count == 0;
    public bool ShowEvidenceEmptyHint => FilteredEvidence.Count == 0;
    public string MappingEmptyHint => MappingTotalCount == 0 ? "暂无标签映射" : "未找到匹配映射";
    public string EvidenceEmptyHint => EvidenceTotalCount == 0 ? "暂无证据记录" : "未找到匹配证据";

    public ObservableCollection<TagMapping> FilteredMappings { get; } = [];
    public ObservableCollection<EvidenceRecord> FilteredEvidence { get; } = [];

    public async Task InitializeAsync()
    {
        await LoadDataAsync();
    }

    public async Task LoadDataAsync()
    {
        IsLoading = true;
        ErrorMessage = null;

        try
        {
            var mappings = await AppViewModel.Instance.TagStore.ListRecentAsync(200);
            var evidence = await AppViewModel.Instance.EvidenceStore.ListRecentAsync(200);

            _allMappings.Clear();
            _allMappings.AddRange(mappings);

            _allEvidence.Clear();
            _allEvidence.AddRange(evidence);

            ReconcileSelections();
            ApplySelectionFlags();
            RefreshFilteredCollections();
            await AppViewModel.Instance.RefreshStatsAsync();
        }
        catch (Exception ex)
        {
            ErrorMessage = $"加载数据库记录失败: {ex.Message}";
        }
        finally
        {
            IsLoading = false;
            NotifyCountersChanged();
        }
    }

    public void ClearErrorMessage()
    {
        ErrorMessage = null;
    }

    public void ClearInfoMessage()
    {
        InfoMessage = null;
    }

    public void EnterMappingsDeleteMode()
    {
        if (!CanEnterMappingsDeleteMode)
        {
            return;
        }

        _scopeBeforeDeleteMode ??= SearchScopeIndex;
        DeleteMode = TagsDeleteMode.Mappings;
        SearchScopeIndex = 1;
        _selectedEvidenceIds.Clear();
        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void EnterEvidenceDeleteMode()
    {
        if (!CanEnterEvidenceDeleteMode)
        {
            return;
        }

        _scopeBeforeDeleteMode ??= SearchScopeIndex;
        DeleteMode = TagsDeleteMode.Evidence;
        SearchScopeIndex = 2;
        _selectedMappingUsernames.Clear();
        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void ExitDeleteMode()
    {
        DeleteMode = TagsDeleteMode.None;

        if (_scopeBeforeDeleteMode is int previousScope)
        {
            SearchScopeIndex = previousScope;
        }
        _scopeBeforeDeleteMode = null;

        _selectedMappingUsernames.Clear();
        _selectedEvidenceIds.Clear();
        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void ToggleMappingSelection(TagMapping mapping)
    {
        if (DeleteMode != TagsDeleteMode.Mappings)
        {
            return;
        }

        if (_selectedMappingUsernames.Contains(mapping.Username))
        {
            _selectedMappingUsernames.Remove(mapping.Username);
        }
        else
        {
            _selectedMappingUsernames.Add(mapping.Username);
        }

        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void ToggleEvidenceSelection(EvidenceRecord record)
    {
        if (DeleteMode != TagsDeleteMode.Evidence)
        {
            return;
        }

        if (_selectedEvidenceIds.Contains(record.Id))
        {
            _selectedEvidenceIds.Remove(record.Id);
        }
        else
        {
            _selectedEvidenceIds.Add(record.Id);
        }

        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void SelectAllInCurrentMode()
    {
        switch (DeleteMode)
        {
            case TagsDeleteMode.Mappings:
                _selectedMappingUsernames.Clear();
                foreach (var mapping in _allMappings)
                {
                    _selectedMappingUsernames.Add(mapping.Username);
                }
                break;
            case TagsDeleteMode.Evidence:
                _selectedEvidenceIds.Clear();
                foreach (var record in _allEvidence)
                {
                    _selectedEvidenceIds.Add(record.Id);
                }
                break;
            default:
                return;
        }

        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public void ClearSelectionInCurrentMode()
    {
        switch (DeleteMode)
        {
            case TagsDeleteMode.Mappings:
                _selectedMappingUsernames.Clear();
                break;
            case TagsDeleteMode.Evidence:
                _selectedEvidenceIds.Clear();
                break;
            default:
                return;
        }

        ApplySelectionFlags();
        RefreshFilteredCollections();
    }

    public int GetCurrentSelectionCount()
    {
        return SelectedCount;
    }

    public bool IsDeleteInputValid(string input)
    {
        return int.TryParse(input.Trim(), out var count) && count == SelectedCount;
    }

    public async Task ExecuteDeleteAsync()
    {
        try
        {
            var deleted = 0;
            switch (DeleteMode)
            {
                case TagsDeleteMode.Mappings:
                    deleted = await AppViewModel.Instance.TagStore.RemoveByUsernamesAsync(_selectedMappingUsernames);
                    InfoMessage = deleted > 0 ? $"已删除 {deleted} 条标签映射" : "未删除任何标签映射";
                    break;
                case TagsDeleteMode.Evidence:
                    deleted = await AppViewModel.Instance.EvidenceStore.RemoveByIdsAsync(_selectedEvidenceIds);
                    InfoMessage = deleted > 0 ? $"已删除 {deleted} 条证据记录" : "未删除任何证据记录";
                    break;
                default:
                    return;
            }

            ErrorMessage = null;
            ExitDeleteMode();
            await LoadDataAsync();
        }
        catch (Exception ex)
        {
            ErrorMessage = $"删除失败: {ex.Message}";
        }
    }

    public string ResolveTagPreview(string username, out bool reusedExisting)
    {
        reusedExisting = false;
        var normalized = username.Trim();
        if (string.IsNullOrWhiteSpace(normalized))
        {
            return "-";
        }

        var existing = _allMappings.FirstOrDefault(item =>
            string.Equals(item.Username, normalized, StringComparison.OrdinalIgnoreCase));
        if (existing is not null)
        {
            reusedExisting = true;
            return existing.Tag;
        }

        var (tag, error) = AwmBridge.SuggestTag(normalized);
        if (error == AwmError.Ok && !string.IsNullOrWhiteSpace(tag))
        {
            return tag;
        }

        return "-";
    }

    public async Task<bool> AddMappingFromUsernameAsync(string username)
    {
        var normalized = username.Trim();
        if (string.IsNullOrWhiteSpace(normalized))
        {
            ErrorMessage = "用户名不能为空";
            return false;
        }

        var previewTag = ResolveTagPreview(normalized, out var reusedExisting);
        if (previewTag == "-")
        {
            ErrorMessage = "无法生成有效 Tag，请更换用户名后重试";
            return false;
        }

        if (reusedExisting)
        {
            InfoMessage = "已存在映射，自动复用";
            ErrorMessage = null;
            return true;
        }

        var inserted = await AppViewModel.Instance.TagStore.SaveIfAbsentAsync(normalized, previewTag);
        if (!inserted)
        {
            InfoMessage = "已存在映射，自动复用";
            ErrorMessage = null;
            return true;
        }

        InfoMessage = $"已新增映射: {normalized} -> {previewTag}";
        ErrorMessage = null;
        await LoadDataAsync();
        return true;
    }

    private void ReconcileSelections()
    {
        var validUsernames = new HashSet<string>(
            _allMappings.Select(item => item.Username),
            StringComparer.OrdinalIgnoreCase);
        _selectedMappingUsernames.RemoveWhere(username => !validUsernames.Contains(username));

        var validEvidenceIds = new HashSet<long>(_allEvidence.Select(item => item.Id));
        _selectedEvidenceIds.RemoveWhere(id => !validEvidenceIds.Contains(id));
    }

    private void ApplySelectionFlags()
    {
        foreach (var mapping in _allMappings)
        {
            mapping.IsSelected = _selectedMappingUsernames.Contains(mapping.Username);
        }

        foreach (var record in _allEvidence)
        {
            record.IsSelected = _selectedEvidenceIds.Contains(record.Id);
        }
    }

    private void RefreshFilteredCollections()
    {
        var query = SearchText.Trim();

        var mappingItems = ShowMappingsPanel
            ? _allMappings.Where(item => MappingMatches(item, query))
            : Enumerable.Empty<TagMapping>();
        ReplaceCollection(FilteredMappings, mappingItems);

        var evidenceItems = ShowEvidencePanel
            ? _allEvidence.Where(item => EvidenceMatches(item, query))
            : Enumerable.Empty<EvidenceRecord>();
        ReplaceCollection(FilteredEvidence, evidenceItems);

        NotifyCountersChanged();
        NotifyDeletePresentationChanged();
    }

    private static bool MappingMatches(TagMapping mapping, string query)
    {
        if (string.IsNullOrEmpty(query))
        {
            return true;
        }

        return mapping.Username.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               mapping.Tag.Contains(query, StringComparison.OrdinalIgnoreCase);
    }

    private static bool EvidenceMatches(EvidenceRecord evidence, string query)
    {
        if (string.IsNullOrEmpty(query))
        {
            return true;
        }

        return evidence.Identity.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               evidence.Tag.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               evidence.FilePath.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               evidence.PcmSha256.Contains(query, StringComparison.OrdinalIgnoreCase);
    }

    private static void ReplaceCollection<T>(ObservableCollection<T> target, IEnumerable<T> source)
    {
        target.Clear();
        foreach (var item in source)
        {
            target.Add(item);
        }
    }

    private void NotifyCountersChanged()
    {
        OnPropertyChanged(nameof(MappingTotalCount));
        OnPropertyChanged(nameof(EvidenceTotalCount));
        OnPropertyChanged(nameof(MappingTotalBadgeText));
        OnPropertyChanged(nameof(EvidenceTotalBadgeText));
        OnPropertyChanged(nameof(MappingSummaryText));
        OnPropertyChanged(nameof(EvidenceSummaryText));
        OnPropertyChanged(nameof(ShowMappingEmptyHint));
        OnPropertyChanged(nameof(ShowEvidenceEmptyHint));
        OnPropertyChanged(nameof(MappingEmptyHint));
        OnPropertyChanged(nameof(EvidenceEmptyHint));
    }

    private void NotifyDeletePresentationChanged()
    {
        OnPropertyChanged(nameof(IsNormalMode));
        OnPropertyChanged(nameof(IsMappingsDeleteMode));
        OnPropertyChanged(nameof(IsEvidenceDeleteMode));
        OnPropertyChanged(nameof(CanOpenAddDialog));
        OnPropertyChanged(nameof(CanEnterMappingsDeleteMode));
        OnPropertyChanged(nameof(CanEnterEvidenceDeleteMode));
        OnPropertyChanged(nameof(CanSelectAll));
        OnPropertyChanged(nameof(CanClearSelection));
        OnPropertyChanged(nameof(CanExecuteDelete));
        OnPropertyChanged(nameof(DeleteTargetLabel));
        OnPropertyChanged(nameof(SelectedCount));
    }
}
