package veins_qos.traffic;

import veins_qos.veins_inet.VeinsInetApplicationBase;

simple VeinsInetCritTrafficApp extends VeinsInetApplicationBase
{
    parameters:
        @class(veins_qos::traffic::VeinsInetCritTrafficApp);
        @signal[voDelay](type=simtime_t);
        @signal[beDelay](type=simtime_t);
        @signal[txVoPk](type=long);
        @signal[txBePk](type=long);
        @signal[rxVoPk](type=long);
        @signal[rxBePk](type=long);
        @statistic[voDelay](title="VO end-to-end delay"; source=voDelay; unit=s; record=mean,stats,vector; interpolationmode=none);
        @statistic[beDelay](title="BE end-to-end delay"; source=beDelay; unit=s; record=mean,stats,vector; interpolationmode=none);
        @statistic[txVoPk](title="VO packets transmitted"; source=txVoPk; record=count);
        @statistic[txBePk](title="BE packets transmitted"; source=txBePk; record=count);
        @statistic[rxVoPk](title="VO packets received"; source=rxVoPk; record=count);
        @statistic[rxBePk](title="BE packets received"; source=rxBePk; record=count);
        // crash behavior
        bool crashEnabled = default(false);
        bool stopVehicleOnCrash = default(false);
        double crashTime @unit(s) = default(0s);
        double crashDuration @unit(s) = default(0s);

        // traffic rates
        double sendInterval @unit(s) = default(0.1s);        // BE period
        double crashSendInterval @unit(s) = default(0.01s);  // VO period while crashActive
        int voBurstPackets = default(20);                    // guaranteed VO burst at crash trigger

        int payloadBytes = default(200);

        @display("i=block/app");
}
