using System;
using System.Threading;

namespace StreamRecorder.Desktop.Services;

public sealed class SingleInstanceService : IDisposable
{
    private const string InstanceMutexName = @"Local\StreamRecorder.SingleInstance";
    private const string ActivationEventName = @"Local\StreamRecorder.Activate";

    private Mutex? _instanceMutex;
    private EventWaitHandle? _activationEvent;
    private RegisteredWaitHandle? _activationRegistration;
    private bool _ownsMutex;

    public bool TryRegisterPrimaryInstance(Action onActivationRequested)
    {
        _instanceMutex = new Mutex(initiallyOwned: false, InstanceMutexName);

        try
        {
            _ownsMutex = _instanceMutex.WaitOne(0, false);
        }
        catch (AbandonedMutexException)
        {
            _ownsMutex = true;
        }

        if (!_ownsMutex)
        {
            return false;
        }

        _activationEvent = new EventWaitHandle(false, EventResetMode.AutoReset, ActivationEventName);
        _activationRegistration = ThreadPool.RegisterWaitForSingleObject(
            _activationEvent,
            static (state, _) => ((Action)state!).Invoke(),
            onActivationRequested,
            Timeout.Infinite,
            false);

        return true;
    }

    public static void TryActivatePrimaryInstance()
    {
        for (var attempt = 0; attempt < 20; attempt++)
        {
            try
            {
                using var activationEvent = EventWaitHandle.OpenExisting(ActivationEventName);
                activationEvent.Set();
                return;
            }
            catch (WaitHandleCannotBeOpenedException)
            {
                Thread.Sleep(100);
            }
        }
    }

    public void Dispose()
    {
        _activationRegistration?.Unregister(null);
        _activationEvent?.Dispose();

        if (_instanceMutex is null)
        {
            return;
        }

        if (_ownsMutex)
        {
            _instanceMutex.ReleaseMutex();
        }

        _instanceMutex.Dispose();
    }
}
