/** Every YT family's AFL parameter table (BTST and ATH), keyed by sub-variant id. */
import {AFL_PARAMS} from './afl-params';
import {AFL_PARAMS_ATH} from './afl-params-ath';
export type {AflParam} from './afl-params';
export const AFL_PARAMS_ALL = {...AFL_PARAMS, ...AFL_PARAMS_ATH};
