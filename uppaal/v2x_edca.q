A[] !(grants.BE_GRANTED && (controller.BLOCKING || controller.SENDING))
A[] ((controller.BLOCKING || controller.SENDING) imply controller.cycle <= maxContinuousBlock)
(controller.BLOCKING && medium_free && voQueueDepth >= voQueueThreshold) --> controller.SENDING
(controller.SENDING && voQueueDepth == 0) --> controller.LISTENING
burstDone --> (controller.LISTENING && medium_free)
