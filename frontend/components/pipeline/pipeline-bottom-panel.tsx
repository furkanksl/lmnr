import { useState } from 'react';

import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { PipelineVersion } from '@/lib/pipeline/types';

import { Skeleton } from '../ui/skeleton';
import PipelineHistory from './pipeline-history';
import PipelineOutputs from './pipeline-outputs';

interface PipelineBottomPanelProps {
  pipelineVersion: PipelineVersion;
  onTraceHover?: (nodeId?: string) => void;
}

export default function PipelineBottomPanel({
  pipelineVersion,
  onTraceHover
}: PipelineBottomPanelProps) {
  const [selectedTab, setSelectedTab] = useState<'runs' | 'history'>('runs');

  return (
    <Tabs
      defaultValue="runs"
      className="h-full z-50 bg-background flex flex-col"
      onValueChange={(value) => setSelectedTab(value as 'runs' | 'history')}
    >
      <div className="flex flex-none border-b z-50 bg-background pl-4">
        <TabsList className="border-none h-12">
          <TabsTrigger value="runs">Run outputs</TabsTrigger>
          {/* <TabsTrigger value="history">History</TabsTrigger> */}
        </TabsList>
      </div>
      <div className="flex-1 w-full">
        <TabsContent
          value="runs"
          className="h-full m-0 relative w-max-0"
          forceMount
          hidden={selectedTab !== 'runs'}
        >
          <div className="absolute inset-0">
            <PipelineOutputs pipelineVersion={pipelineVersion} />
          </div>
        </TabsContent>
        <TabsContent
          value="history"
          className="h-full m-0 relative w-max-0"
          forceMount
          hidden={selectedTab !== 'history'}
        >
          {pipelineVersion?.id ? (
            <div className="absolute inset-0">
              <PipelineHistory
                pipelineVersion={pipelineVersion}
                onTraceHover={onTraceHover}
              />
            </div>
          ) : (
            <Skeleton className="w-full h-full" />
          )}
        </TabsContent>
      </div>
    </Tabs>
  );
}
