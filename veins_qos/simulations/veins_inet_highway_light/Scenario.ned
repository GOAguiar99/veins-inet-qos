package veins_qos.simulations.veins_inet_highway_light;

import inet.environment.common.PhysicalEnvironment;
import inet.physicallayer*.wireless.ieee80211.packetlevel.Ieee80211DimensionalRadioMedium;
import inet.visualizer*.common.IntegratedVisualizer;
import org.car2x.veins.visualizer.roads.RoadsCanvasVisualizer;
import org.car2x.veins.visualizer.roads.RoadsOsgVisualizer;
import veins_qos.veins_inet.VeinsInetCar;
import veins_qos.veins_inet.VeinsInetManager;

network Scenario
{
    parameters:
        bool useOsg = default(false);
        @display("bgb=319,384");
    submodules:
        radioMedium: Ieee80211DimensionalRadioMedium {
            @display("p=64,224");
        }
        manager: VeinsInetManager {
            @display("p=192,320");
        }
        visualizer: IntegratedVisualizer {
            @display("p=64,320");
        }
        physicalEnvironment: PhysicalEnvironment {
            @display("p=192,224");
        }
        roadsCanvasVisualizer: RoadsCanvasVisualizer {
            @display("p=64,416");
        }
        roadsOsgVisualizer: RoadsOsgVisualizer if useOsg {
            @display("p=192,416");
        }
        node[0]: VeinsInetCar;
}
