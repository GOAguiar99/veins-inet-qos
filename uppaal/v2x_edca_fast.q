// Q1 Safety (fast): BE is never granted while suppression is active.
A[] !(grants.BE_GRANTED && (controller.BLOCKING || controller.SENDING))

// Q2 Progress (fast): blocking with VO demand reaches sending.
controller.BLOCKING && medium_free && voQueueDepth >= voQueueThreshold --> controller.SENDING

// Q3 Recovery (fast): controller returns to listening after VO clears.
controller.SENDING && voQueueDepth == 0 --> controller.LISTENING

// Q4 End (fast): eventually listening after workload completion.
burstDone --> controller.LISTENING
