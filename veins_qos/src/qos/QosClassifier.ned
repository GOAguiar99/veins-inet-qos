package veins_qos.qos;

import inet.linklayer.common.IIeee8021dQosClassifier;

simple QosClassifier like IIeee8021dQosClassifier
{
    parameters:
        @display("i=block/classifier");
    gates:
        input in;
        output out;
}
