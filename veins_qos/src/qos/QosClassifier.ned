package veins_qos.qos;

import inet.linklayer.common.IIeee8021dQosClassifier;

simple QosClassifier like IIeee8021dQosClassifier
{
    parameters:
        @class(veins_qos::qos::QosClassifier);
        int crashDscp = default(46);
        int defaultUp = default(0);           // inet::UP_BE
        @display("i=block/classifier");
    gates:
        input in;
        output out;
}
