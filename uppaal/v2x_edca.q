// Q1 Safety: BE must never be granted while controller is suppressing BE.
A[] !(grants.BE_GRANTED && (controller.BLOCKING || controller.SENDING))

// Q2 Safety: blocking/sending cycle must remain bounded.
A[] (!(controller.BLOCKING || controller.SENDING) || controller.cycle <= maxContinuousBlock)

// Q3 Progress: if VO demand exists and medium is free while blocking, controller can reach SENDING.
A[] (!(controller.BLOCKING && medium_free && voQueueDepth >= voQueueThreshold) || A<> controller.SENDING)

// Q4 Recovery: once VO queue is empty during SENDING, eventually return to LISTENING.
A[] (!(controller.SENDING && voQueueDepth == 0) || A<> controller.LISTENING)

// Q5 End condition: after burst generation is complete, controller+medium can reach idle-ready posture.
A[] (!burstDone || A<> (controller.LISTENING && medium_free))
