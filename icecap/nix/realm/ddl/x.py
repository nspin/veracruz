from icedl.common import GenericElfComponent
from icedl.realm import BaseRealmComposition


class RuntimeManager(GenericElfComponent):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        channel = self.composition.extern_ring_buffer('realm_{}_channel_ring_buffer'.format(self.composition.realm_id()), size=1<<21)

        self._arg = {
            'host_ring_buffer': self.map_ring_buffer(channel),
            }

    def arg_json(self):
        return self._arg


class Composition(BaseRealmComposition):

    def compose(self):
        self.component(RuntimeManager, 'runtime_manager')


Composition.run()
