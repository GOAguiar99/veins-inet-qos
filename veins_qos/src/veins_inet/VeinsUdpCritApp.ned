package veins_qos.veins_inet;

import inet.applications.udpapp.UdpBasicApp;

module VeinsUdpCritApp extends UdpBasicApp
{
    parameters:
        string criticality = default("low"); // low|medium|high|carCrash
        bool crashEnabled = default(false);
        double crashTime @unit(s) = default(-1s);       // when to crash
        double crashDuration @unit(s) = default(10s);   // how long to stop (SUMO), 0 = no resume
        int crashDscp = default(46);                    // DSCP when crashed (EF=46 recommended)
        bool stopVehicleOnCrash = default(true);        // try to stop SUMO vehicle
}