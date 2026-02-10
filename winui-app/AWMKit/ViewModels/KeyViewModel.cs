using CommunityToolkit.Mvvm.ComponentModel;
using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// Key management page state model.
/// </summary>
public sealed partial class KeyViewModel : ObservableObject
{
    private readonly AppViewModel _appViewModel = AppViewModel.Instance;

    private bool _isBusy;
    public bool IsBusy
    {
        get => _isBusy;
        private set => SetProperty(ref _isBusy, value);
    }

    private int _selectedSlot;
    public int SelectedSlot
    {
        get => _selectedSlot;
        set => SetProperty(ref _selectedSlot, Math.Clamp(value, 0, 31));
    }

    public ObservableCollection<int> SlotOptions { get; } = new();

    public bool KeyAvailable => _appViewModel.KeyAvailable;
    public string KeySourceLabel => _appViewModel.KeySourceLabel;
    public int ActiveKeySlot => _appViewModel.ActiveKeySlot;
    public string ActiveKeySlotText => $"当前激活槽位：{ActiveKeySlot}";
    public string KeyStatusText => KeyAvailable ? "已配置" : "未配置";
    public string SlotHintText => "当前版本嵌入仍写槽位 0，槽位切换将在后续协议生效阶段接入。";

    public KeyViewModel()
    {
        for (var i = 0; i <= 31; i++)
        {
            SlotOptions.Add(i);
        }

        _appViewModel.PropertyChanged += AppViewModelOnPropertyChanged;
    }

    public async Task InitializeAsync()
    {
        await _appViewModel.RefreshRuntimeStatusAsync();
        SelectedSlot = _appViewModel.ActiveKeySlot;
        RaiseComputedProperties();
    }

    public async Task GenerateKeyAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            await _appViewModel.GenerateKeyCommand.ExecuteAsync(null);
            await _appViewModel.RefreshRuntimeStatusAsync();
            SelectedSlot = _appViewModel.ActiveKeySlot;
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    public async Task DeleteKeyAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            await _appViewModel.DeleteKeyCommand.ExecuteAsync(null);
            await _appViewModel.RefreshRuntimeStatusAsync();
            SelectedSlot = _appViewModel.ActiveKeySlot;
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    public async Task SaveSelectedSlotAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            await _appViewModel.SetActiveKeySlotAsync(SelectedSlot);
            await _appViewModel.RefreshActiveKeySlotAsync();
            SelectedSlot = _appViewModel.ActiveKeySlot;
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    private void AppViewModelOnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(AppViewModel.KeyAvailable):
            case nameof(AppViewModel.KeySourceLabel):
            case nameof(AppViewModel.ActiveKeySlot):
                if (e.PropertyName == nameof(AppViewModel.ActiveKeySlot))
                {
                    SelectedSlot = _appViewModel.ActiveKeySlot;
                }

                RaiseComputedProperties();
                break;
        }
    }

    private void RaiseComputedProperties()
    {
        OnPropertyChanged(nameof(KeyAvailable));
        OnPropertyChanged(nameof(KeySourceLabel));
        OnPropertyChanged(nameof(ActiveKeySlot));
        OnPropertyChanged(nameof(ActiveKeySlotText));
        OnPropertyChanged(nameof(KeyStatusText));
    }
}
