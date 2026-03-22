// Q1 Safety: BE must never be granted while controller is suppressing BE.
A[] !(grants.BE_GRANTED && (controller.BLOCKING || controller.SENDING))

// Q2 Safety: if maxContinuousBlock is enabled, blocking/sending cycle must remain bounded.
A[] (maxContinuousBlock == 0 || !(controller.BLOCKING || controller.SENDING) || controller.cycle <= maxContinuousBlock)

// Q3 Progress: if VO demand exists and medium is free while suppressing, SENDING is eventually reached.
controller.BLOCKING && medium_free && voQueueDepth >= voQueueThreshold --> controller.SENDING

// Q4 Recovery: once SENDING has no pending VO, eventually return to LISTENING.
controller.SENDING && voQueueDepth == 0 --> controller.LISTENING

// Q5 End condition: after crash workload completes, controller+medium can reach idle-ready posture.
burstDone --> (controller.LISTENING && medium_free)
