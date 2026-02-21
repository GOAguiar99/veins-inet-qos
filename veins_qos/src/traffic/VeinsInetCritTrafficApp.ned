package veins_qos.traffic;

import veins_inet.VeinsInetApplicationBase;

simple VeinsInetCritTrafficApp extends VeinsInetApplicationBase
{
    parameters:
        // crash behavior
        bool crashEnabled = default(false);
        bool stopVehicleOnCrash = default(false);
        double crashTime @unit(s) = default(0s);
        double crashDuration @unit(s) = default(0s);

        // traffic rates
        double sendInterval @unit(s) = default(0.1s);        // BE period
        double crashSendInterval @unit(s) = default(0.01s);  // VO period while crashActive

        int payloadBytes = default(200B);

        @display("i=block/app");
}