"use client";

import { FormEvent, useState } from "react";

import ChatHeader from "@/components/chat/header";
import Messages from "@/components/chat/messages";
import MultimodalInput from "@/components/chat/multimodal-input";
import { ChatMessage } from "@/components/chat/types";
import { useAgentChat } from "@/components/chat/useAgentChat";

interface ChatProps {
  chatId: string;
  userId: string;
  initialMessages: ChatMessage[];
}

const Chat = ({ chatId, userId, initialMessages }: ChatProps) => {
  const [model, setModel] = useState<string>("claude-3-7-sonnet-latest");
  const [enableThinking, setEnableThinking] = useState(true);
  const { messages, handleSubmit, stop, isLoading, input, setInput } = useAgentChat({
    id: chatId,
    initialMessages,
    userId,
  });

  const onSubmit = (e?: FormEvent<HTMLFormElement>) => {
    if (e) {
      e.preventDefault();
    }
    handleSubmit(e, { model, enableThinking });
  };

  return (
    <div className="flex flex-col min-w-0 h-dvh bg-background">
      <ChatHeader />
      <Messages isLoading={isLoading} messages={messages} />
      <form onSubmit={onSubmit} className="flex mx-auto px-4 bg-background pb-4 md:pb-6 gap-2 w-full md:max-w-3xl">
        <MultimodalInput
          enableThinking={enableThinking}
          onEnableThinkingChange={setEnableThinking}
          model={model}
          onModelChange={setModel}
          onSubmit={() => onSubmit()}
          stop={stop}
          isLoading={isLoading}
          value={input}
          onChange={setInput}
        />
      </form>
    </div>
  );
};

export default Chat;
